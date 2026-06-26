--TEST--
sasso: canonicalize() receives $fromImport=true for @import, false for @use
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;
use Sasso\Importer;
use Sasso\ImporterResult;

$importer = new class implements Importer {
    public array $seen = [];
    public function canonicalize(string $url, bool $fromImport, ?string $containingUrl = null): ?string {
        $this->seen[$url] = $fromImport;
        return "c:$url";
    }
    public function load(string $canonicalUrl): ?ImporterResult {
        return new ImporterResult('.a { b: c; }');
    }
};

(new Compiler())->setImporter($importer)->compile('@use "viaUse"; @import "viaImport";');

var_dump($importer->seen['viaUse']);
var_dump($importer->seen['viaImport']);
?>
--EXPECT--
bool(false)
bool(true)
