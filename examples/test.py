#!/usr/bin/env python3

from unittest import TestCase
import os
import errno


class IOFailure(Exception): pass


def func_under_test(filename):
    try:
        with open(filename, 'rt') as f:
            return f.read()
    except EnvironmentError:
        raise IOFailure()


def clear():
    '''clears the `libfaultinj` environment settings
    '''
    items = [k for k in os.environ.keys() if 'LIBFAULTINJ' in k]
    for item in items:
        del os.environ[item]


class T(TestCase):
    FILE_TO_FAIL_ON = './somefile.txt'

    def setUp(self):
        clear()
        with open(T.FILE_TO_FAIL_ON, 'wt') as f:
            f.write('file contents')

        assert 'LD_PRELOAD' in os.environ

    def tearDown(self):
        os.unlink(T.FILE_TO_FAIL_ON)

    def test_expect_fail(self):
        os.environ['LIBFAULTINJ_ERROR_PATH'] = T.FILE_TO_FAIL_ON
        os.environ['LIBFAULTINJ_ERROR_OPEN_ERRNO'] = str(errno.ENOMEM)

        with self.assertRaises(IOFailure):
            func_under_test(T.FILE_TO_FAIL_ON)

    def test_expect_success(self):
        func_under_test(T.FILE_TO_FAIL_ON)
