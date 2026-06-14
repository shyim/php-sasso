#![cfg_attr(windows, feature(abi_vectorcall))]
//! PHP extension exposing the [`sasso`] pure-Rust SCSS → CSS compiler to PHP.
//!
//! It provides a `Sasso\Compiler` class with a fluent builder API and a
//! `Sasso\Importer` interface for userland `@import`/`@use` resolution. Compile
//! failures are surfaced as a `Sasso\CompileException`.

use std::path::PathBuf;

use ext_php_rs::prelude::*;
use ext_php_rs::convert::IntoZval;
use ext_php_rs::exception::{self, PhpException};
use ext_php_rs::types::{ZendObject, Zval};
use ext_php_rs::zend::ExecutorGlobals;
use sasso_compiler::{compile, FsImporter, Importer as SassoImporter, Options, OutputStyle, Syntax};

/// Output style: human-readable, indented CSS (the default).
const STYLE_EXPANDED: i64 = 0;
/// Output style: minified, single-line CSS.
const STYLE_COMPRESSED: i64 = 1;

/// Input syntax: brace/semicolon SCSS (the default).
const SYNTAX_SCSS: i64 = 0;
/// Input syntax: indented `.sass`.
const SYNTAX_SASS: i64 = 1;
/// Input syntax: plain CSS (Sass features rejected, values emitted verbatim).
const SYNTAX_CSS: i64 = 2;

fn output_style(style: i64) -> Result<OutputStyle, PhpException> {
    match style {
        STYLE_EXPANDED => Ok(OutputStyle::Expanded),
        STYLE_COMPRESSED => Ok(OutputStyle::Compressed),
        other => Err(PhpException::default(format!(
            "Sasso: invalid output style {other}; expected Compiler::STYLE_EXPANDED or STYLE_COMPRESSED"
        ))),
    }
}

fn input_syntax(syntax: i64) -> Result<Syntax, PhpException> {
    match syntax {
        SYNTAX_SCSS => Ok(Syntax::Scss),
        SYNTAX_SASS => Ok(Syntax::Sass),
        SYNTAX_CSS => Ok(Syntax::Css),
        other => Err(PhpException::default(format!(
            "Sasso: invalid syntax {other}; expected Compiler::SYNTAX_SCSS, SYNTAX_SASS or SYNTAX_CSS"
        ))),
    }
}

/// Bridges a userland `Sasso\Importer` object to sasso's [`SassoImporter`]
/// trait. `resolve` calls back into the PHP VM synchronously (sasso compiles on
/// this same thread, so no cross-thread concern). An optional [`FsImporter`]
/// handles load paths as a fallback when the PHP importer returns `null`.
struct PhpImporter<'a> {
    object: &'a ZendObject,
    fallback: Option<FsImporter>,
}

impl SassoImporter for PhpImporter<'_> {
    fn resolve(&self, path: &str) -> Option<String> {
        // If a userland exception is already pending (from a prior resolve),
        // stop — don't call back into PHP again. The pending exception is
        // surfaced by `run` after `compile` unwinds.
        if ExecutorGlobals::has_exception() {
            return None;
        }
        let returned = self.object.try_call_method("resolve", vec![&path]).ok();
        // If the call threw, leave the exception pending and abort resolution;
        // `run` rethrows it after `compile` unwinds.
        if ExecutorGlobals::has_exception() {
            return None;
        }
        returned
            .and_then(|zval| zval.string())
            .or_else(|| self.fallback.as_ref().and_then(|f| f.resolve(path)))
    }
}

/// Run a compile with the resolved options, mapping a sasso `Error` to a
/// `Sasso\CompileException`.
fn run(
    source: &str,
    style: i64,
    syntax: i64,
    unicode: bool,
    url: Option<&str>,
    load_paths: &[String],
    php_importer: Option<&ZendObject>,
) -> PhpResult<String> {
    let mut options = Options::new()
        .with_style(output_style(style)?)
        .with_syntax(input_syntax(syntax)?)
        .with_unicode(unicode);

    if let Some(url) = url {
        options = options.with_url(url);
    }

    // The importer must outlive the `compile` call; build it here so the
    // borrow in `Options` is valid for the whole call. A userland importer
    // takes precedence (with the FS load paths as its fallback); otherwise a
    // plain `FsImporter` is used when any load paths were configured.
    let fs;
    let php;
    if let Some(object) = php_importer {
        let fallback = if load_paths.is_empty() {
            None
        } else {
            Some(FsImporter::new(load_paths.iter().map(PathBuf::from).collect()))
        };
        php = PhpImporter { object, fallback };
        options = options.with_importer(&php);
    } else if !load_paths.is_empty() {
        fs = FsImporter::new(load_paths.iter().map(PathBuf::from).collect());
        options = options.with_importer(&fs);
    }

    let result = compile(source, &options);

    // A userland importer may have thrown; that exception was left pending and
    // resolution aborted, so sasso reports a generic import failure. Surface
    // the *original* PHP exception instead by re-throwing it. We return `Ok`
    // (not `Err`) so ext-php-rs does not overwrite the pending exception with
    // one of its own — PHP checks `EG(exception)` after the call regardless of
    // the returned value, so the original exception is what the caller sees.
    if php_importer.is_some() {
        if let Some(exception) = ExecutorGlobals::take_exception() {
            if let Ok(zval) = exception.into_zval(false) {
                let _ = exception::throw_object(zval);
            }
            return Ok(String::new());
        }
    }

    result.map_err(|e| PhpException::from_class::<CompileException>(e.to_string()))
}

/// A userland resolver for `@import` / `@use` / `@forward`.
///
/// Implement this in PHP and pass an instance to `Compiler::setImporter()` to
/// control where partials come from (a database, a virtual filesystem, an
/// archive, …). `resolve()` is given the unquoted URL exactly as written in the
/// source (e.g. `"base"` for `@import "base"`) and must return the partial's
/// SCSS/Sass source, or `null` if it cannot be found.
///
/// ```php
/// class ArrayImporter implements Sasso\Importer {
///     public function __construct(private array $files) {}
///     public function resolve(string $url): ?string {
///         return $this->files[$url] ?? null;
///     }
/// }
/// ```
#[php_interface]
#[php(name = "Sasso\\Importer")]
pub trait Importer {
    /// Resolve a partial URL to its SCSS/Sass source, or `null` if not found.
    fn resolve(&self, url: String) -> Option<String>;
}

/// Thrown when SCSS/Sass compilation fails (a parse or evaluation error).
///
/// The message is sasso's diagnostic — a byte-exact snippet when a `url`/path
/// is configured, otherwise the legacy `Error: <msg> (line:col)` one-liner.
#[php_class]
#[php(name = "Sasso\\CompileException")]
#[php(extends(ce = ext_php_rs::zend::ce::exception, stub = "\\Exception"))]
#[derive(Default)]
pub struct CompileException;

/// A fluent SCSS → CSS compiler backed by the Rust `sasso` crate.
///
/// ```php
/// $css = (new Sasso\Compiler())
///     ->setStyle(Sasso\Compiler::STYLE_COMPRESSED)
///     ->addImportPath(__DIR__ . '/scss')
///     ->compile('@import "base"; .a { color: red; &:hover { color: blue; } }');
/// ```
#[php_class]
#[php(name = "Sasso\\Compiler")]
pub struct Compiler {
    style: i64,
    syntax: i64,
    unicode: bool,
    url: Option<String>,
    load_paths: Vec<String>,
    // The userland `Sasso\Importer` object, held as an owned object zval (a
    // `shallow_clone` bumps its refcount so it lives as long as the compiler).
    importer: Option<Zval>,
}

impl Default for Compiler {
    fn default() -> Self {
        Compiler {
            style: STYLE_EXPANDED,
            syntax: SYNTAX_SCSS,
            unicode: true,
            url: None,
            load_paths: Vec::new(),
            importer: None,
        }
    }
}

#[php_impl]
impl Compiler {
    /// Output style: human-readable, indented CSS (the default).
    const STYLE_EXPANDED: i64 = STYLE_EXPANDED;
    /// Output style: minified, single-line CSS.
    const STYLE_COMPRESSED: i64 = STYLE_COMPRESSED;
    /// Input syntax: brace/semicolon SCSS (the default).
    const SYNTAX_SCSS: i64 = SYNTAX_SCSS;
    /// Input syntax: indented `.sass`.
    const SYNTAX_SASS: i64 = SYNTAX_SASS;
    /// Input syntax: plain CSS.
    const SYNTAX_CSS: i64 = SYNTAX_CSS;

    /// Create a compiler with default options (expanded, SCSS, Unicode diagnostics).
    pub fn __construct() -> Self {
        Compiler::default()
    }

    /// Set the output style (one of the `STYLE_*` constants). Returns `$this`.
    ///
    /// An out-of-range value is validated (and throws) at `compile()` time.
    pub fn set_style(&mut self, style: i64) -> &mut Self {
        self.style = style;
        self
    }

    /// Set the input syntax (one of the `SYNTAX_*` constants). Returns `$this`.
    ///
    /// An out-of-range value is validated (and throws) at `compile()` time.
    pub fn set_syntax(&mut self, syntax: i64) -> &mut Self {
        self.syntax = syntax;
        self
    }

    /// Toggle Unicode box-drawing glyphs in diagnostics (`false` = ASCII).
    pub fn set_unicode(&mut self, unicode: bool) -> &mut Self {
        self.unicode = unicode;
        self
    }

    /// Set the input's path/URL as it should appear in diagnostics, enabling
    /// byte-exact error snippets. Pass `null` to disable.
    pub fn set_url(&mut self, url: Option<String>) -> &mut Self {
        self.url = url;
        self
    }

    /// Append a load path searched for `@import`/`@use`/`@forward` partials.
    pub fn add_import_path(&mut self, path: String) -> &mut Self {
        self.load_paths.push(path);
        self
    }

    /// Replace all load paths at once.
    pub fn set_import_paths(&mut self, paths: Vec<String>) -> &mut Self {
        self.load_paths = paths;
        self
    }

    /// Set a userland `Sasso\Importer` that resolves `@import`/`@use`/`@forward`
    /// partials. It is consulted first; any configured load paths act as a
    /// fallback when its `resolve()` returns `null`. Pass `null` to clear it.
    ///
    /// Throws if `$importer` is not an instance of `Sasso\Importer`.
    pub fn set_importer(&mut self, importer: &Zval) -> &mut Self {
        if importer.is_null() {
            self.importer = None;
            return self;
        }
        let importer_ce =
            <PhpInterfaceImporter as ext_php_rs::class::RegisteredClass>::get_metadata().ce();
        match importer.object() {
            Some(object) if object.instance_of(importer_ce) => {
                // Hold an owned reference (bumps the object refcount) so it
                // survives until the next setImporter / compiler drop.
                self.importer = Some(importer.shallow_clone());
            }
            _ => {
                let _ = PhpException::default(
                    "Sasso: importer must implement Sasso\\Importer".into(),
                )
                .throw();
            }
        }
        self
    }

    /// Compile `source` with the configured options, returning CSS.
    ///
    /// Throws `Sasso\CompileException` on a parse or evaluation error.
    pub fn compile(&self, source: String) -> PhpResult<String> {
        let php_importer = self.importer.as_ref().and_then(Zval::object);
        run(
            &source,
            self.style,
            self.syntax,
            self.unicode,
            self.url.as_deref(),
            &self.load_paths,
            php_importer,
        )
    }
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .interface::<PhpInterfaceImporter>()
        .class::<CompileException>()
        .class::<Compiler>()
}
