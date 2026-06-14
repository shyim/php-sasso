dnl config.m4 for the sasso extension.
dnl
dnl sasso is a Rust extension built with ext-php-rs, so the actual compilation
dnl is done by cargo rather than the usual C toolchain. This config.m4 wires the
dnl phpize/configure/make flow PIE (and `pecl install`) expect to a cargo build:
dnl `make` runs `cargo build --release` and the resulting cdylib is copied into
dnl the build's `modules/` directory as `sasso.so`.

PHP_ARG_ENABLE([sasso],
  [whether to enable the sasso SCSS compiler extension],
  [AS_HELP_STRING([--enable-sasso], [Enable the sasso SCSS compiler extension])],
  [yes])

if test "$PHP_SASSO" != "no"; then
  AC_PATH_PROG(CARGO, cargo, no)
  if test "$CARGO" = "no"; then
    AC_MSG_ERROR([cargo (the Rust toolchain) is required to build the sasso extension; install it from https://rustup.rs])
  fi

  PHP_NEW_EXTENSION([sasso], [], [$ext_shared])

  dnl The source lives outside the phpize build tree; record where so the cargo
  dnl rule below can find Cargo.toml regardless of the configure working dir.
  CARGO_MANIFEST_DIR=$abs_srcdir
  PHP_SUBST([CARGO_MANIFEST_DIR])
  PHP_SUBST([CARGO])

  dnl Build the cdylib with cargo and drop it into modules/ as sasso.so before
  dnl PHP's own `install-modules` rule copies modules/* into the extension dir.
  dnl cargo emits libsasso.dylib on macOS and libsasso.so on Linux, so
  dnl copy whichever the build produced.
  cat >> Makefile.fragments <<'EOF'

cargo_build:
	cd $(CARGO_MANIFEST_DIR) && $(CARGO) build --release
	test -d modules || mkdir modules
	cp $(CARGO_MANIFEST_DIR)/target/release/libsasso.dylib modules/sasso.so 2>/dev/null || \
		cp $(CARGO_MANIFEST_DIR)/target/release/libsasso.so modules/sasso.so

all: cargo_build
build-modules: cargo_build
EOF
fi
