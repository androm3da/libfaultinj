all: test

LIBS := LD_LIBRARY_PATH=../target/debug LD_PRELOAD=libfaultinj.so

test:
	$(LIBS) ruby test.rb
	$(LIBS) python3 -m unittest discover --verbose
