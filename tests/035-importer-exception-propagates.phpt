--TEST--
sasso: an exception thrown from the importer propagates out of compile() unchanged
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;
use Sasso\Importer;
use Sasso\ImporterResult;

class ImporterBoom extends \RuntimeException {}

$importer = new class implements Importer {
    public function canonicalize(string $url, bool $fromImport, ?string $containingUrl = null): ?string {
        throw new ImporterBoom('db is down');
    }
    public function load(string $canonicalUrl): ?ImporterResult {
        return null;
    }
};

try {
    (new Compiler())->setImporter($importer)->compile('@use "x";');
    echo "no exception\n";
} catch (\Throwable $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}
?>
--EXPECT--
ImporterBoom
db is down
