

`libfaultinj` is a fault-injection library.

== Concept

Use `LD_PRELOAD` to load `libfaultinj` in front of your platform's C library.  It will 
conditionally execute a delay before executing the operation or return an error instead
of executing the operation.

TODO: testing w/`DYLD_INSERT_LIBRARIES` on OS X or other similar platforms.

== Usage
Set `LIBFAULTINJ_DELAY_PATH` to the directory or filename to be delayed.

... to be continued ...
