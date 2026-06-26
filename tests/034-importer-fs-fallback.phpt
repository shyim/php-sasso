--TEST--
sasso: load paths are the fallback when canonicalize() returns null
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;
use Sasso\Importer;
use Sasso\ImporterResult;

$dir = __DIR__ . '/_fixtures_034';
@mkdir($dir);
file_put_contents($dir . '/_brand.scss', '$c: #0a0;');

$importer = new class implements Importer {
    // Resolves nothing: forces the filesystem fallback.
    public function canonicalize(string $url, bool $fromImport, ?string $containingUrl = null): ?string {
        return null;
    }
    public function load(string $canonicalUrl): ?ImporterResult {
        return null;
    }
};

echo (new Compiler())
    ->setImporter($importer)
    ->addImportPath($dir)
    ->setStyle(Compiler::STYLE_COMPRESSED)
    ->compile('@use "brand"; .x { color: brand.$c; }'), "\n";
?>
--CLEAN--
<?php
$dir = __DIR__ . '/_fixtures_034';
@unlink($dir . '/_brand.scss');
@rmdir($dir);
?>
--EXPECT--
.x{color:#0a0}
