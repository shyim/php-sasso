--TEST--
sasso: a nested import's canonicalize() gets the parent's canonical $containingUrl
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;
use Sasso\Importer;
use Sasso\ImporterResult;

$importer = new class implements Importer {
    public array $containing = [];
    public function canonicalize(string $url, bool $fromImport, ?string $containingUrl = null): ?string {
        $this->containing[$url] = $containingUrl;
        return "c:$url";
    }
    public function load(string $canonicalUrl): ?ImporterResult {
        $url = substr($canonicalUrl, strlen('c:'));
        // "root" pulls in "child"; "child" is leaf CSS.
        return new ImporterResult($url === 'root' ? '@use "child";' : '.y { z: 1; }');
    }
};

(new Compiler())
    ->setImporter($importer)
    ->setUrl('entry.scss')   // the entry's canonical URL; becomes root's containingUrl
    ->compile('@use "root";');

var_dump($importer->containing['root']);   // resolved against the entry url
var_dump($importer->containing['child']);  // resolved against root's canonical url
?>
--EXPECT--
string(10) "entry.scss"
string(6) "c:root"
