.PHONY: build test clean
build:
	idris2 --build network-intent.ipkg
test: build
	python3 tests/run.py
	idris2 --build core-tests.ipkg
	./build/core-tests/exec/core-tests
	python3 scripts/check_typestate.py
	python3 scripts/check_release.py
	python3 scripts/check_core_probes.py
clean:
	rm -rf build
