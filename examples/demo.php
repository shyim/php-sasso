<?php
// Run with the extension loaded, e.g.:
//   php -dextension=../target/release/libsasso.dylib examples/demo.php

declare(strict_types=1);

use Sasso\Compiler;
use Sasso\CompileException;
use Sasso\Importer;

$scss = <<<'SCSS'
$brand: #336699;
.card {
    color: $brand;
    &:hover { color: lighten($brand, 10%); }
    .title { font-weight: bold; }
}
SCSS;

echo "── expanded ──\n", (new Compiler())->compile($scss), "\n";
echo "── compressed ──\n", (new Compiler())->setStyle(Compiler::STYLE_COMPRESSED)->compile($scss), "\n\n";

try {
    (new Compiler())->setUrl('demo.scss')->compile('.broken { color: ');
} catch (CompileException $e) {
    echo "── caught CompileException ──\n", $e->getMessage(), "\n";
}

// A custom importer resolving partials from an in-memory map.
$importer = new class implements Importer {
    private array $files = ['base' => '$brand: #e91e63;'];
    public function canonicalize(string $url, bool $fromImport, ?string $containingUrl = null): ?string {
        return isset($this->files[$url]) ? "mem:$url" : null;
    }
    public function load(string $canonicalUrl): ?Sasso\ImporterResult {
        $key = substr($canonicalUrl, strlen('mem:'));
        return new Sasso\ImporterResult($this->files[$key]);
    }
};

echo "\n── custom importer ──\n", (new Compiler())
    ->setImporter($importer)
    ->setStyle(Compiler::STYLE_COMPRESSED)
    ->compile('@use "base"; .btn { color: base.$brand; }'), "\n";
