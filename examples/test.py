#!/usr/bin/env python3

from unittest import TestCase
import os
import errno
import time


class IOFailure(Exception):
    pass


def func_under_test(filename):
    try:
        with open(filename, 'rt') as f:
            return f.read()
    except EnvironmentError:
        raise IOFailure()



def cleanup_env():
    '''clears the `libfaultinj` environment settings
    '''
    items = [k for k in os.environ.keys() if 'LIBFAULTINJ' in k]
    for item in items:
        del os.environ[item]


class FileTest(TestCase):
    FILE_TO_FAIL_ON = './somefile.txt'

    def setUp(self):
        cleanup_env()
        with open(FileTest.FILE_TO_FAIL_ON, 'wt') as f:
            f.write('file contents')

        assert 'LD_PRELOAD' in os.environ

    def tearDown(self):
        os.unlink(FileTest.FILE_TO_FAIL_ON)

    def test_expect_fail(self):
        os.environ['LIBFAULTINJ_ERROR_PATH'] = FileTest.FILE_TO_FAIL_ON
        os.environ['LIBFAULTINJ_ERROR_OPEN_ERRNO'] = str(errno.ENOMEM)

        with self.assertRaises(IOFailure):
            func_under_test(FileTest.FILE_TO_FAIL_ON)

    def test_expect_success(self):
        func_under_test(FileTest.FILE_TO_FAIL_ON)


class NetTest(TestCase):
    # Value should represent the injected delay duration, in seconds.
    #   This value should be significantly larger than MAX_WRITE_DUR_SEC
    #   so that there's no risk of a false negative of the injected delay
    #   test.
    INJECTED_WRITE_DELAY_DUR_SEC = 10

    # Value should represent the maximum duration to send a single
    #   byte over a TCP connection in the 'established' state using
    #   the loopback interface to the test harness.
    MAX_WRITE_DUR_SEC = 1.

    def setUp(self):
        try:
            import socketserver
        except ImportError:
            import SocketServer as socketserver

        self.local_addr = '127.0.0.1'
        self.port = 0
        addr = (self.local_addr, self.port)

        import multiprocessing
        request_occurred = multiprocessing.Event()
        request_duration = multiprocessing.Queue()

        class RequestHandler(socketserver.BaseRequestHandler):
            def handle(self):
                t0 = time.time()
                data = self.request.recv(1)
                request_occurred.set()

                dur = time.time() - t0
                request_duration.put(dur)

                return

        self.request_occurred = request_occurred
        self.request_duration = request_duration
        self.server = socketserver.ForkingTCPServer(addr, RequestHandler)

        import threading
        self.server_listen_thread = threading.Thread(
            target=self.server.serve_forever)
        assert 'LD_PRELOAD' in os.environ

        self.server_listen_thread.start()

    def _connect_and_send(self, byte_array_):
        from contextlib import closing
        import socket

        with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as sock:
            t0 = time.time()
            sock.connect(self.server.server_address)
            connect_dur_sec = time.time() - t0

            t0 = time.time()
            sock.sendall(byte_array_)
            write_dur_sec = time.time() - t0

        return connect_dur_sec, write_dur_sec

    def test_send_delay(self):
        os.environ['LIBFAULTINJ_DELAY_PATH'] = 'ignore'
        connect_dur_sec, write_dur_sec = self._connect_and_send(b'T')
        assert write_dur_sec < NetTest.MAX_WRITE_DUR_SEC

        os.environ['LIBFAULTINJ_DELAY_SEND_MS'] = str(int(NetTest.INJECTED_WRITE_DELAY_DUR_SEC * 1000))
        os.environ['LIBFAULTINJ_DELAY_PATH'] = self.local_addr
        connect_dur_sec, send_dur_sec = self._connect_and_send(b't')
        assert send_dur_sec > NetTest.INJECTED_WRITE_DELAY_DUR_SEC

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.server_listen_thread.join()

        cleanup_env()

#        print('request occurred', self.request_occurred.is_set())
