--TEST--
sasso: addImportPath() resolves partials from the filesystem (no userland importer)
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;

$dir = __DIR__ . '/_fixtures_040';
@mkdir($dir);
file_put_contents($dir . '/_vars.scss', '$brand: #369;');

echo (new Compiler())
    ->addImportPath($dir)
    ->setStyle(Compiler::STYLE_COMPRESSED)
    ->compile('@use "vars"; .a { color: vars.$brand; }'), "\n";
?>
--CLEAN--
<?php
$dir = __DIR__ . '/_fixtures_040';
@unlink($dir . '/_vars.scss');
@rmdir($dir);
?>
--EXPECT--
.a{color:#369}
