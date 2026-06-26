#![cfg_attr(windows, feature(abi_vectorcall))]
//! PHP extension exposing the [`sasso`] pure-Rust SCSS → CSS compiler to PHP.
//!
//! It provides a `Sasso\Compiler` class with a fluent builder API and a
//! `Sasso\Importer` interface for userland `@import`/`@use` resolution. Compile
//! failures are surfaced as a `Sasso\CompileException`.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;

use ext_php_rs::prelude::*;
use ext_php_rs::convert::IntoZval;
use ext_php_rs::exception::{self, PhpException};
use ext_php_rs::types::{ZendClassObject, ZendObject, Zval};
use ext_php_rs::zend::ExecutorGlobals;
use sasso_compiler::{
    compile, CanonicalUrl, CanonicalizeContext, FsImporter, Importer as SassoImporter,
    ImporterError, ImporterResult, Options, OutputStyle, Syntax,
};

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

/// Bridges a userland `Sasso\Importer` object to sasso's two-phase
/// [`SassoImporter`] trait, exposing dart-sass's `canonicalize`/`load` split
/// directly to PHP. `canonicalize($url, $fromImport, $containingUrl)` maps a
/// (possibly relative, extension-less) URL to a stable canonical string;
/// `load($canonicalUrl)` then returns a `Sasso\ImporterResult` (or `null`).
/// Both call back into the PHP VM synchronously (sasso compiles on this same
/// thread, so no cross-thread concern). An optional [`FsImporter`] handles load
/// paths as a fallback when the PHP importer returns `null` from either phase.
///
/// Canonical strings minted by the PHP importer are tracked in `php_canonicals`
/// so `load` knows whether to route a given key back into PHP or to the
/// filesystem fallback (whose canonical keys are absolute paths it owns).
struct PhpImporter<'a> {
    object: &'a ZendObject,
    fallback: Option<FsImporter>,
    // Canonical strings the PHP importer returned from `canonicalize`. `load`
    // routes these back into PHP; anything else goes to the fallback.
    // `RefCell` because the trait methods take `&self`.
    php_canonicals: RefCell<HashSet<String>>,
}

impl SassoImporter for PhpImporter<'_> {
    fn canonicalize(
        &self,
        url: &str,
        ctx: &CanonicalizeContext<'_>,
    ) -> Result<Option<CanonicalUrl>, ImporterError> {
        // If a userland exception is already pending (from a prior call), stop —
        // don't call back into PHP again. The pending exception is surfaced by
        // `run` after `compile` unwinds.
        if ExecutorGlobals::has_exception() {
            return Ok(None);
        }
        let from_import = ctx.from_import;
        let containing = ctx.containing_url.map(CanonicalUrl::as_str);
        let returned = self
            .object
            .try_call_method("canonicalize", vec![&url, &from_import, &containing])
            .ok();
        // If the call threw, leave the exception pending and abort; `run`
        // rethrows it after `compile` unwinds.
        if ExecutorGlobals::has_exception() {
            return Ok(None);
        }
        if let Some(canonical) = returned.and_then(|zval| zval.string()) {
            self.php_canonicals.borrow_mut().insert(canonical.clone());
            return Ok(Some(CanonicalUrl::new(canonical)));
        }
        // The PHP importer doesn't handle this URL; fall back to the filesystem
        // load paths, if any.
        match &self.fallback {
            Some(f) => f.canonicalize(url, ctx),
            None => Ok(None),
        }
    }

    fn load(
        &self,
        canonical: &CanonicalUrl,
    ) -> Result<Option<ImporterResult>, ImporterError> {
        // A canonical key the PHP importer minted is loaded back through PHP;
        // anything else came from the filesystem fallback, so delegate there.
        if !self.php_canonicals.borrow().contains(canonical.as_str()) {
            return match &self.fallback {
                Some(f) => f.load(canonical),
                None => Ok(None),
            };
        }

        if ExecutorGlobals::has_exception() {
            return Ok(None);
        }
        let key = canonical.as_str();
        let returned = self.object.try_call_method("load", vec![&key]).ok();
        if ExecutorGlobals::has_exception() {
            return Ok(None);
        }
        let Some(object) = returned.as_ref().and_then(Zval::object) else {
            // `null` (or a non-object) => the file can no longer be found.
            return Ok(None);
        };
        // The return value must be a `Sasso\ImporterResult`; pull the native
        // struct straight out of the class object rather than reading PHP
        // properties reflectively.
        let result = ZendClassObject::<ImporterResultClass>::from_zend_obj(object).ok_or_else(|| {
            ImporterError {
                message: "Sasso\\Importer::load() must return a Sasso\\ImporterResult or null"
                    .to_owned(),
            }
        })?;
        Ok(Some(ImporterResult {
            contents: result.contents.clone(),
            syntax: importer_syntax(result.syntax)?,
            source_map_url: result.source_map_url.clone(),
        }))
    }
}

/// Map a `SYNTAX_*` integer from a `Sasso\ImporterResult` to a [`Syntax`].
fn importer_syntax(syntax: i64) -> Result<Syntax, ImporterError> {
    match syntax {
        SYNTAX_SCSS => Ok(Syntax::Scss),
        SYNTAX_SASS => Ok(Syntax::Sass),
        SYNTAX_CSS => Ok(Syntax::Css),
        other => Err(ImporterError {
            message: format!(
                "Sasso\\ImporterResult::$syntax is {other}; expected Compiler::SYNTAX_SCSS, SYNTAX_SASS or SYNTAX_CSS"
            ),
        }),
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
        php = PhpImporter {
            object,
            fallback,
            php_canonicals: RefCell::new(HashSet::new()),
        };
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

/// A userland resolver for `@import` / `@use` / `@forward`, mirroring dart-sass's
/// two-phase importer protocol.
///
/// Implement this in PHP and pass an instance to `Compiler::setImporter()` to
/// control where partials come from (a database, a virtual filesystem, an
/// archive, …). Resolution happens in two phases:
///
/// 1. `canonicalize()` maps a (possibly relative, extension-less) URL — exactly
///    as written in the source, e.g. `"base"` for `@import "base"` — to a stable
///    canonical string that identifies the partial. It MUST NOT load the file.
///    Two URLs that canonicalize to the same string are the SAME partial (it is
///    the module-cache / dedup key). Return `null` if this importer cannot
///    resolve the URL. `$fromImport` is `true` for `@import` (which also allows
///    import-only files), `false` for `@use`/`@forward`; `$containingUrl` is the
///    canonical URL of the stylesheet making the request (or `null`), against
///    which relative URLs resolve.
/// 2. `load()` is then given a canonical string this importer returned and
///    fetches its source as a `Sasso\ImporterResult` (or `null` if it can no
///    longer be found).
///
/// ```php
/// class ArrayImporter implements Sasso\Importer {
///     public function __construct(private array $files) {}
///     public function canonicalize(string $url, bool $fromImport, ?string $containingUrl = null): ?string {
///         return isset($this->files[$url]) ? "array:$url" : null;
///     }
///     public function load(string $canonicalUrl): ?Sasso\ImporterResult {
///         $key = substr($canonicalUrl, strlen('array:'));
///         return new Sasso\ImporterResult($this->files[$key]);
///     }
/// }
/// ```
#[php_interface]
#[php(name = "Sasso\\Importer")]
pub trait Importer {
    /// Map a URL to its canonical identity, or `null` if not handled. MUST NOT
    /// load the file.
    fn canonicalize(
        &self,
        url: String,
        from_import: bool,
        containing_url: Option<String>,
    ) -> Option<String>;

    /// Load the source for a canonical string previously returned by
    /// `canonicalize()`. Returns a `Sasso\ImporterResult`, or `null` if it can no
    /// longer be found.
    fn load(&self, canonical_url: String) -> Option<Zval>;
}

/// The source an [`Importer::load`] produced — dart-sass's `ImporterResult`.
///
/// ```php
/// $r = new Sasso\ImporterResult(
///     '.a { color: red; }',
///     Sasso\Compiler::SYNTAX_SCSS, // optional, defaults to SCSS
/// );
/// ```
#[php_class]
#[php(name = "Sasso\\ImporterResult")]
pub struct ImporterResultClass {
    /// The stylesheet source text.
    #[php(prop)]
    pub contents: String,
    /// The syntax `contents` is parsed with (a `Compiler::SYNTAX_*` constant;
    /// defaults to `SYNTAX_SCSS`).
    #[php(prop)]
    pub syntax: i64,
    /// The URL recorded for this source in generated source maps; `null` falls
    /// back to the canonical URL. Exposed in PHP as `$sourceMapUrl`.
    #[php(prop)]
    pub source_map_url: Option<String>,
}

#[php_impl]
impl ImporterResultClass {
    /// Construct an importer result from `contents`, an optional `SYNTAX_*`
    /// constant (default SCSS), and an optional source-map URL.
    pub fn __construct(
        contents: String,
        syntax: Option<i64>,
        source_map_url: Option<String>,
    ) -> Self {
        ImporterResultClass {
            contents,
            syntax: syntax.unwrap_or(SYNTAX_SCSS),
            source_map_url,
        }
    }
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
    /// fallback when its `canonicalize()` returns `null`. Pass `null` to clear it.
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
        .class::<ImporterResultClass>()
        .class::<CompileException>()
        .class::<Compiler>()
}
