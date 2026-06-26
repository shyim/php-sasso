--TEST--
sasso: SYNTAX_SASS parses the indented syntax
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;

echo (new Compiler())
    ->setSyntax(Compiler::SYNTAX_SASS)
    ->setStyle(Compiler::STYLE_COMPRESSED)
    ->compile(".a\n  color: red"), "\n";
?>
--EXPECT--
.a{color:red}
