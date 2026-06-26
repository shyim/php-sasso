--TEST--
sasso: setImporter() rejects an object that does not implement Sasso\Importer
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
use Sasso\Compiler;

try {
    (new Compiler())->setImporter(new stdClass());
    echo "no exception\n";
} catch (\Throwable $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}
?>
--EXPECT--
Exception
Sasso: importer must implement Sasso\Importer
