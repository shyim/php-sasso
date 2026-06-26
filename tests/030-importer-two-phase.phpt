--TEST--
sasso: two-phase userland importer (canonicalize + load) resolves @use
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;
use Sasso\Importer;
use Sasso\ImporterResult;

$importer = new class implements Importer {
    public array $files = ['base' => '$brand: #e91e63;'];
    public array $log = [];

    public function canonicalize(string $url, bool $fromImport, ?string $containingUrl = null): ?string {
        $this->log[] = "canonicalize($url, " . ($fromImport ? 'true' : 'false') . ")";
        return isset($this->files[$url]) ? "mem:$url" : null;
    }

    public function load(string $canonicalUrl): ?ImporterResult {
        $this->log[] = "load($canonicalUrl)";
        $key = substr($canonicalUrl, strlen('mem:'));
        return isset($this->files[$key]) ? new ImporterResult($this->files[$key]) : null;
    }
};

$css = (new Compiler())
    ->setImporter($importer)
    ->setStyle(Compiler::STYLE_COMPRESSED)
    ->compile('@use "base"; .btn { color: base.$brand; }');

echo $css, "\n";
echo implode("\n", $importer->log), "\n";
?>
--EXPECT--
.btn{color:#e91e63}
canonicalize(base, false)
load(mem:base)
