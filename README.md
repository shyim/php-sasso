# php-sasso

A PHP extension that compiles **SCSS / Sass → CSS** natively, in-process, with
no Node, no Dart VM, and no subprocess. It is a thin [ext-php-rs] binding around
[`sasso`] — a pure-Rust, zero-dependency dart-sass alternative that matches
current dart-sass byte-for-byte on the subset it implements.

[ext-php-rs]: https://github.com/davidcole1340/ext-php-rs
[`sasso`]: https://crates.io/crates/sasso

The loaded extension is named **`sasso`** (`extension=sasso`, `php -m` shows
`sasso`, `extension_loaded('sasso')`); `php-sasso` is just the repository name.

## Requirements

- PHP 8.2+ (developed against 8.5)
- A Rust toolchain (`cargo`)
- `libclang` for bindgen (macOS: `brew install llvm`)

## Install with PIE

The package is [PIE]-compatible. With a Rust toolchain available, PIE builds the
extension from source (via `phpize`/`configure`/`make`, which shell out to cargo)
and installs it:

```console
$ pie install shyim/sasso
```

Then enable it in your `php.ini`:

```ini
extension=sasso
```

[PIE]: https://github.com/php/pie

## Build & install manually

```console
$ ./install.sh
```

This builds `--release` and copies the module into `$(php-config --extension-dir)`
as `sasso.so`. Then enable it in your `php.ini` with `extension=sasso.so`.

Equivalently, the PIE/pecl build flow works directly:

```console
$ phpize && ./configure && make && make install
```

To try it without installing:

```console
$ cargo build --release
$ php -dextension=target/release/libsasso.dylib  your_script.php   # macOS
$ php -dextension=target/release/libsasso.so     your_script.php   # Linux
```

## Usage

The API is object-oriented: a single fluent `Sasso\Compiler` class.

### Fluent compiler

```php
<?php
use Sasso\Compiler;
use Sasso\CompileException;

$css = (new Compiler())
    ->setStyle(Compiler::STYLE_COMPRESSED)
    ->addImportPath(__DIR__ . '/scss')   // load path for @import / @use / @forward
    ->setUrl('app.scss')                 // enables byte-exact error snippets
    ->compile('@use "base"; .x { color: base.$brand; }');

try {
    (new Compiler())->setUrl('input.scss')->compile('.a { color: ');
} catch (CompileException $e) {
    // Error: unexpected end of input in value
    //   ╷
    // 1 │ .a { color:
    //   │            ^
    //   ╵
    //   input.scss 1:13  root stylesheet
    echo $e->getMessage();
}
```

### Custom importer

`@import` / `@use` / `@forward` can be resolved from anywhere — a database, an
archive, a virtual filesystem — by implementing the `Sasso\Importer` interface
and passing an instance to `setImporter()`:

```php
<?php
use Sasso\Compiler;
use Sasso\Importer;

class ArrayImporter implements Importer {
    public function __construct(private array $files) {}

    // Given the unquoted URL as written (e.g. "base" for @import "base"),
    // return its SCSS/Sass source, or null if it cannot be found.
    public function resolve(string $url): ?string {
        return $this->files[$url] ?? null;
    }
}

$css = (new Compiler())
    ->setImporter(new ArrayImporter([
        'base'   => '$brand: #e91e63;',
        'mixins' => '@mixin big { font-size: 2rem; }',
    ]))
    ->compile('@use "base"; @import "mixins"; .btn { color: base.$brand; @include big; }');
```

The importer is consulted first; any `addImportPath()` load paths act as a
fallback when `resolve()` returns `null`. If `resolve()` returns `null` and
nothing else resolves the URL, the import fails with a `CompileException`.

If `resolve()` **throws**, that exception propagates out of `compile()`
unchanged — same class, code and message — so you can surface a real error
(a missing DB row, an I/O failure) instead of a generic "stylesheet not found":

```php
class DbImporter implements Importer {
    public function resolve(string $url): ?string {
        $row = $this->db->find($url);              // may throw DbException
        return $row?->source;                      // null => normal import error
    }
}

try {
    (new Compiler())->setImporter(new DbImporter($db))->compile($scss);
} catch (DbException $e) {
    // the original DbException, not a CompileException
}
```

### Indented `.sass` syntax

```php
echo (new Compiler())
    ->setSyntax(Compiler::SYNTAX_SASS)
    ->compile(".a\n  color: red");
```

## API

### `Sasso\Compiler`

| Method | Description |
| --- | --- |
| `__construct()` | Defaults: expanded, SCSS, Unicode diagnostics. |
| `setStyle(int $style): static` | `STYLE_EXPANDED` or `STYLE_COMPRESSED`. |
| `setSyntax(int $syntax): static` | `SYNTAX_SCSS`, `SYNTAX_SASS` or `SYNTAX_CSS`. |
| `setUnicode(bool $on): static` | Unicode vs ASCII diagnostic glyphs. |
| `setUrl(?string $url): static` | Display path for diagnostics (enables snippets). |
| `addImportPath(string $path): static` | Append an `@import`/`@use` load path. |
| `setImportPaths(array $paths): static` | Replace all load paths. |
| `setImporter(?Sasso\Importer $importer): static` | Userland resolver (load paths are the fallback); `null` clears. |
| `compile(string $source): string` | Compile; throws `Sasso\CompileException`. |

### `Sasso\Importer` (interface)

| Method | Description |
| --- | --- |
| `resolve(string $url): ?string` | Return the partial's SCSS/Sass source for `$url`, or `null` if not found. |

All setters return `$this` for chaining. Invalid `STYLE_*` / `SYNTAX_*` values are
reported (as a thrown exception) at `compile()` time.

### Constants (on `Sasso\Compiler`)

`STYLE_EXPANDED = 0`, `STYLE_COMPRESSED = 1`,
`SYNTAX_SCSS = 0`, `SYNTAX_SASS = 1`, `SYNTAX_CSS = 2`.

## License

MIT OR Apache-2.0, matching `sasso`.
