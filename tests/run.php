<?php
/**
 * Run the .phpt test suite against the locally built sasso extension.
 *
 * Usage:
 *   php tests/run.php            # build (if needed) + run every tests/*.phpt
 *   php tests/run.php tests/030-importer-two-phase.phpt   # run specific tests
 *
 * It locates PHP's bundled run-tests.php, the freshly built cdylib
 * (target/release/libsasso.{dylib,so}), and runs the suite with that extension
 * loaded — so you do not have to install the extension into your php.ini first.
 */

$root = dirname(__DIR__);

// 1. Locate the built extension; build it with cargo if it is missing.
$candidates = [
    $root . '/target/release/libsasso.dylib',
    $root . '/target/release/libsasso.so',
];
$ext = null;
foreach ($candidates as $c) {
    if (is_file($c)) { $ext = $c; break; }
}
if ($ext === null) {
    fwrite(STDERR, "Building extension (cargo build --release)…\n");
    passthru('cargo build --release --manifest-path ' . escapeshellarg($root . '/Cargo.toml'), $code);
    if ($code !== 0) {
        fwrite(STDERR, "cargo build failed\n");
        exit(1);
    }
    foreach ($candidates as $c) {
        if (is_file($c)) { $ext = $c; break; }
    }
}
if ($ext === null) {
    fwrite(STDERR, "Could not find a built libsasso cdylib under target/release/\n");
    exit(1);
}

// 2. Locate PHP's bundled run-tests.php (ships under the build/ dir).
$runTests = getenv('RUN_TESTS_PHP') ?: null;
if ($runTests === null) {
    $guesses = [];
    if (defined('PHP_BINARY')) {
        $prefix = dirname(dirname(PHP_BINARY));
        $guesses[] = $prefix . '/lib/php/build/run-tests.php';
        $guesses[] = $prefix . '/lib/php/run-tests.php';
    }
    // `php-config --prefix` is the most reliable locator across distros.
    $cfgPrefix = trim((string) @shell_exec('php-config --prefix 2>/dev/null'));
    if ($cfgPrefix !== '') {
        $guesses[] = $cfgPrefix . '/lib/php/build/run-tests.php';
    }
    foreach ($guesses as $g) {
        if (is_file($g)) { $runTests = $g; break; }
    }
}
if ($runTests === null || !is_file($runTests)) {
    fwrite(STDERR, "Could not locate run-tests.php. Set RUN_TESTS_PHP to its path.\n");
    exit(1);
}

// 3. Which tests? Anything passed on the CLI, else the whole tests/ dir.
$args = array_slice($argv, 1);
$targets = $args !== [] ? $args : [__DIR__];

// 4. Run. The child test processes are configured via TEST_PHP_EXECUTABLE /
//    TEST_PHP_ARGS: -n ignores the system php.ini (so a previously installed
//    sasso build can't shadow it) and -d extension=… loads the freshly built
//    cdylib instead.
$php = PHP_BINARY;
$childArgs = '-n -d extension=' . $ext;
putenv('TEST_PHP_EXECUTABLE=' . $php);
putenv('TEST_PHP_ARGS=' . $childArgs);

$cmd = array_merge(
    [$php, '-n', $runTests, '-q', '--show-diff', '-p', $php],
    $targets,
);
$escaped = implode(' ', array_map('escapeshellarg', $cmd));

fwrite(STDERR, "Extension: {$ext}\nrun-tests: {$runTests}\n\n");
passthru($escaped, $code);
exit($code);
