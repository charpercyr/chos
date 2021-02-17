
use chos_test::*;

use crate::println;

#[allow(improper_ctypes)]
extern {
    static __TEST_CASES_START: *const TestCase;
    static __TEST_CASES_END: *const TestCase;
}

pub fn test_runner(_: &[&dyn Fn()]) {
    unsafe {
        let start = &__TEST_CASES_START as *const *const _ as *const TestCase;
        let end = &__TEST_CASES_END as *const *const _ as *const TestCase;
        println!("{:p} {:p}", start, end);
        let tests = get_tests(start, end);
        for test in tests {
            println!("Running {}", test.name);
        }
    };
}
