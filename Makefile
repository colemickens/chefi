all: 
	./build/build.sh
.PHONY: all

test:
	./build/test.sh
.PHONY: test

verify:
	./build/verify-all.sh
.PHONY: verify