--TEST--
sasso: an out-of-range STYLE_* value is rejected at compile() time
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;

try {
    (new Compiler())->setStyle(99)->compile('.a { b: c; }');
    echo "no exception\n";
} catch (\Throwable $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}
?>
--EXPECT--
Exception
Sasso: invalid output style 99; expected Compiler::STYLE_EXPANDED or STYLE_COMPRESSED
