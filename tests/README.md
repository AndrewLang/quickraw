# Tests for quickraw

Please notice that sample raw files needed are not included in the repository.

You can download your own tests files in [https://raw.pixls.us/](https://raw.pixls.us/).

Sample file list:
* sample0.ARW


/**
 *
$env:QUICKRAW_TEST_CANON_CR2="D:\Photos\Brands\Cannon\EOS 1Ds Mark II\Canon - EOS-1Ds Mark II - RAW (3_2).CR2"
$env:QUICKRAW_TEST_CANON_CR3="D:\Photos\Brands\Cannon\EOS R6 Mark III\163A6102.CR3"
cargo test -q --test test_canon_thumbnail

 */