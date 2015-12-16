
require 'test/unit'
require 'fileutils'


class StringTest < Test::Unit::TestCase
  @@file_name = "./thisfile.txt"

  def teardown
    ENV::delete('LIBFAULTINJ_ERROR_PATH')
    ENV::delete('LIBFAULTINJ_ERROR_OPEN_ERRNO')

    FileUtils.touch(@@file_name)
    File.unlink(@@file_name)
  end

  def test_expect_success
    assert_raise( Errno::ENOENT ) { x = File.read(@@file_name) }
  end

  def test_expect_fail
    FileUtils.touch(@@file_name)
    ENV['LIBFAULTINJ_ERROR_PATH'] = @@file_name
    ENV['LIBFAULTINJ_ERROR_OPEN_ERRNO'] = "2"

    assert_raise( Errno::ENOENT ) { x = File.read(@@file_name) }
  end

end
