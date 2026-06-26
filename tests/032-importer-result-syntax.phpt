--TEST--
sasso: ImporterResult can declare a per-partial syntax (SYNTAX_SASS)
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;
use Sasso\Importer;
use Sasso\ImporterResult;

$importer = new class implements Importer {
    public function canonicalize(string $url, bool $fromImport, ?string $containingUrl = null): ?string {
        return "s:$url";
    }
    public function load(string $canonicalUrl): ?ImporterResult {
        // Indented Sass source returned from an otherwise-SCSS compile.
        return new ImporterResult(".partial\n  color: red", Compiler::SYNTAX_SASS);
    }
};

echo (new Compiler())
    ->setImporter($importer)
    ->setStyle(Compiler::STYLE_COMPRESSED)
    ->compile('@import "p";'), "\n";
?>
--EXPECT--
.partial{color:red}
