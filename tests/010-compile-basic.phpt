--TEST--
sasso: basic compile with nesting (expanded, the default)
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;

echo (new Compiler())
    ->compile('.a { color: red; &:hover { color: blue; } }');
?>
--EXPECT--
.a {
  color: red;
}
.a:hover {
  color: blue;
}
