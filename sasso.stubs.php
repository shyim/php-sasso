<?php

// Stubs for sasso

namespace Sasso {
    /**
     * A userland resolver for `@import` / `@use` / `@forward`, mirroring dart-sass's
     * two-phase importer protocol.
     *
     * Implement this in PHP and pass an instance to `Compiler::setImporter()` to
     * control where partials come from (a database, a virtual filesystem, an
     * archive, …). Resolution happens in two phases:
     *
     * 1. `canonicalize()` maps a (possibly relative, extension-less) URL — exactly
     *    as written in the source, e.g. `"base"` for `@import "base"` — to a stable
     *    canonical string that identifies the partial. It MUST NOT load the file.
     *    Two URLs that canonicalize to the same string are the SAME partial (it is
     *    the module-cache / dedup key). Return `null` if this importer cannot
     *    resolve the URL. `$fromImport` is `true` for `@import` (which also allows
     *    import-only files), `false` for `@use`/`@forward`; `$containingUrl` is the
     *    canonical URL of the stylesheet making the request (or `null`), against
     *    which relative URLs resolve.
     * 2. `load()` is then given a canonical string this importer returned and
     *    fetches its source as a `Sasso\ImporterResult` (or `null` if it can no
     *    longer be found).
     *
     * ```php
     * class ArrayImporter implements Sasso\Importer {
     *     public function __construct(private array $files) {}
     *     public function canonicalize(string $url, bool $fromImport, ?string $containingUrl = null): ?string {
     *         return isset($this->files[$url]) ? "array:$url" : null;
     *     }
     *     public function load(string $canonicalUrl): ?Sasso\ImporterResult {
     *         $key = substr($canonicalUrl, strlen('array:'));
     *         return new Sasso\ImporterResult($this->files[$key]);
     *     }
     * }
     * ```
     */
    interface Importer {
        /**
         * Map a URL to its canonical identity, or `null` if not handled. MUST NOT
         * load the file.
         */
        public function canonicalize(string $url, bool $from_import, ?string $containing_url = null): ?string;

        /**
         * Load the source for a canonical string previously returned by
         * `canonicalize()`. Returns a `Sasso\ImporterResult`, or `null` if it can no
         * longer be found.
         */
        public function load(string $canonical_url): mixed;
    }

    /**
     * The source an [`Importer::load`] produced — dart-sass's `ImporterResult`.
     *
     * ```php
     * $r = new Sasso\ImporterResult(
     *     '.a { color: red; }',
     *     Sasso\Compiler::SYNTAX_SCSS, // optional, defaults to SCSS
     * );
     * ```
     */
    class ImporterResult {
        /**
         * The stylesheet source text.
         */
        public string$contents;

        /**
         * The syntax `contents` is parsed with (a `Compiler::SYNTAX_*` constant;
         * defaults to `SYNTAX_SCSS`).
         */
        public int$syntax;

        /**
         * The URL recorded for this source in generated source maps; `null` falls
         * back to the canonical URL. Exposed in PHP as `$sourceMapUrl`.
         */
        public string$sourceMapUrl = null;

        /**
         * Construct an importer result from `contents`, an optional `SYNTAX_*`
         * constant (default SCSS), and an optional source-map URL.
         */
        public function __construct(string $contents, ?int $syntax = null, ?string $source_map_url = null) {}
    }

    /**
     * Thrown when SCSS/Sass compilation fails (a parse or evaluation error).
     *
     * The message is sasso's diagnostic — a byte-exact snippet when a `url`/path
     * is configured, otherwise the legacy `Error: <msg> (line:col)` one-liner.
     */
    class CompileException extends \Exception {
        public function __construct() {}
    }

    /**
     * A fluent SCSS → CSS compiler backed by the Rust `sasso` crate.
     *
     * ```php
     * $css = (new Sasso\Compiler())
     *     ->setStyle(Sasso\Compiler::STYLE_COMPRESSED)
     *     ->addImportPath(__DIR__ . '/scss')
     *     ->compile('@import "base"; .a { color: red; &:hover { color: blue; } }');
     * ```
     */
    class Compiler {
        /**
         * Output style: human-readable, indented CSS (the default).
         */
        const STYLE_EXPANDED = 0;

        /**
         * Output style: minified, single-line CSS.
         */
        const STYLE_COMPRESSED = 1;

        /**
         * Input syntax: brace/semicolon SCSS (the default).
         */
        const SYNTAX_SCSS = 0;

        /**
         * Input syntax: indented `.sass`.
         */
        const SYNTAX_SASS = 1;

        /**
         * Input syntax: plain CSS.
         */
        const SYNTAX_CSS = 2;

        /**
         * Set the output style (one of the `STYLE_*` constants). Returns `$this`.
         *
         * An out-of-range value is validated (and throws) at `compile()` time.
         */
        public function setStyle(int $style): \Sasso\Compiler {}

        /**
         * Set the input syntax (one of the `SYNTAX_*` constants). Returns `$this`.
         *
         * An out-of-range value is validated (and throws) at `compile()` time.
         */
        public function setSyntax(int $syntax): \Sasso\Compiler {}

        /**
         * Toggle Unicode box-drawing glyphs in diagnostics (`false` = ASCII).
         */
        public function setUnicode(bool $unicode): \Sasso\Compiler {}

        /**
         * Set the input's path/URL as it should appear in diagnostics, enabling
         * byte-exact error snippets. Pass `null` to disable.
         */
        public function setUrl(?string $url = null): \Sasso\Compiler {}

        /**
         * Append a load path searched for `@import`/`@use`/`@forward` partials.
         */
        public function addImportPath(string $path): \Sasso\Compiler {}

        /**
         * Replace all load paths at once.
         */
        public function setImportPaths(array $paths): \Sasso\Compiler {}

        /**
         * Set a userland `Sasso\Importer` that resolves `@import`/`@use`/`@forward`
         * partials. It is consulted first; any configured load paths act as a
         * fallback when its `canonicalize()` returns `null`. Pass `null` to clear it.
         *
         * Throws if `$importer` is not an instance of `Sasso\Importer`.
         */
        public function setImporter(mixed $importer): \Sasso\Compiler {}

        /**
         * Compile `source` with the configured options, returning CSS.
         *
         * Throws `Sasso\CompileException` on a parse or evaluation error.
         */
        public function compile(string $source): string {}

        /**
         * Create a compiler with default options (expanded, SCSS, Unicode diagnostics).
         */
        public function __construct() {}
    }
}
