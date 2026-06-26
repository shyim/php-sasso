--TEST--
sasso: Sasso\ImporterResult constructor defaults (syntax => SCSS, sourceMapUrl => null)
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;
use Sasso\ImporterResult;

$r = new ImporterResult('.a { b: c; }');
var_dump($r->contents);
var_dump($r->syntax === Compiler::SYNTAX_SCSS);
var_dump($r->sourceMapUrl);

$r2 = new ImporterResult('x', Compiler::SYNTAX_CSS, 'https://example/map');
var_dump($r2->syntax === Compiler::SYNTAX_CSS);
var_dump($r2->sourceMapUrl);
?>
--EXPECT--
string(12) ".a { b: c; }"
bool(true)
NULL
bool(true)
string(19) "https://example/map"
