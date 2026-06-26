--TEST--
sasso: STYLE_COMPRESSED minifies the output
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;

$css = (new Compiler())
    ->setStyle(Compiler::STYLE_COMPRESSED)
    ->compile('.a { color: red; &:hover { color: blue; } }');

echo $css, "\n";
?>
--EXPECT--
.a{color:red}.a:hover{color:blue}
