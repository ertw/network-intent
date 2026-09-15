.PHONY: build test test-runtime test-browser clean
build:
	idris2 --build network-intent.ipkg
test: build
	python3 tests/run.py
	python3 tests/router.py
	python3 tests/wireless.py
	idris2 --build core-tests.ipkg
	./build/core-tests/exec/core-tests
	python3 scripts/check_typestate.py
	python3 scripts/check_release.py
	python3 scripts/check_core_probes.py
test-runtime: build
	cargo test --workspace
test-browser:
	python3 scripts/check_browser_parity.py
clean:
	rm -rf build
