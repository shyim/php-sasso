<?php

// Stubs for sasso

namespace Sasso {
    /**
     * A userland resolver for `@import` / `@use` / `@forward`.
     *
     * Implement this in PHP and pass an instance to `Compiler::setImporter()` to
     * control where partials come from (a database, a virtual filesystem, an
     * archive, …). `resolve()` is given the unquoted URL exactly as written in the
     * source (e.g. `"base"` for `@import "base"`) and must return the partial's
     * SCSS/Sass source, or `null` if it cannot be found.
     *
     * ```php
     * class ArrayImporter implements Sasso\Importer {
     *     public function __construct(private array $files) {}
     *     public function resolve(string $url): ?string {
     *         return $this->files[$url] ?? null;
     *     }
     * }
     * ```
     */
    interface Importer {
        /**
         * Resolve a partial URL to its SCSS/Sass source, or `null` if not found.
         */
        public function resolve(string $url): ?string;
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
         * fallback when its `resolve()` returns `null`. Pass `null` to clear it.
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
