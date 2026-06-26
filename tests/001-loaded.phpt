--TEST--
sasso: extension is loaded and exposes its public API
--SKIPIF--
<?php if (!extension_loaded('sasso')) echo 'skip sasso not loaded'; ?>
--FILE--
<?php
var_dump(extension_loaded('sasso'));
var_dump(class_exists('Sasso\\Compiler'));
var_dump(class_exists('Sasso\\CompileException'));
var_dump(class_exists('Sasso\\ImporterResult'));
var_dump(interface_exists('Sasso\\Importer'));
var_dump(is_a('Sasso\\CompileException', 'Exception', true));
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
