--TEST--
sasso: a parse/eval error throws Sasso\CompileException
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;

try {
    (new Compiler())->compile('.broken { color: ');
    echo "no exception\n";
} catch (\Sasso\CompileException $e) {
    echo "caught CompileException\n";
    var_dump($e instanceof \Exception);
}
?>
--EXPECT--
caught CompileException
bool(true)
