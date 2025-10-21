
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad 53
    .quad app_0_start
    .quad app_1_start
    .quad app_2_start
    .quad app_3_start
    .quad app_4_start
    .quad app_5_start
    .quad app_6_start
    .quad app_7_start
    .quad app_8_start
    .quad app_9_start
    .quad app_10_start
    .quad app_11_start
    .quad app_12_start
    .quad app_13_start
    .quad app_14_start
    .quad app_15_start
    .quad app_16_start
    .quad app_17_start
    .quad app_18_start
    .quad app_19_start
    .quad app_20_start
    .quad app_21_start
    .quad app_22_start
    .quad app_23_start
    .quad app_24_start
    .quad app_25_start
    .quad app_26_start
    .quad app_27_start
    .quad app_28_start
    .quad app_29_start
    .quad app_30_start
    .quad app_31_start
    .quad app_32_start
    .quad app_33_start
    .quad app_34_start
    .quad app_35_start
    .quad app_36_start
    .quad app_37_start
    .quad app_38_start
    .quad app_39_start
    .quad app_40_start
    .quad app_41_start
    .quad app_42_start
    .quad app_43_start
    .quad app_44_start
    .quad app_45_start
    .quad app_46_start
    .quad app_47_start
    .quad app_48_start
    .quad app_49_start
    .quad app_50_start
    .quad app_51_start
    .quad app_52_start
    .quad app_52_end

    .global _app_names
    _app_names:
    .string "adder"
    .string "adder_atomic"
    .string "adder_mutex_blocking"
    .string "adder_mutex_spin"
    .string "adder_peterson_spin"
    .string "adder_peterson_yield"
    .string "adder_simple_spin"
    .string "adder_simple_yield"
    .string "barrier_condvar"
    .string "barrier_fail"
    .string "cat"
    .string "cmdline_args"
    .string "condsync_condvar"
    .string "condsync_sem"
    .string "count_lines"
    .string "early_exit"
    .string "early_exit2"
    .string "eisenberg"
    .string "exit"
    .string "fantastic_text"
    .string "filetest_simple"
    .string "forktest"
    .string "forktest2"
    .string "forktest_simple"
    .string "forktree"
    .string "hello_world"
    .string "huge_write"
    .string "infloop"
    .string "initproc"
    .string "initproc_test"
    .string "matrix"
    .string "mpsc_sem"
    .string "peterson"
    .string "phil_din_mutex"
    .string "pipe_large_test"
    .string "pipetest"
    .string "priv_csr"
    .string "priv_inst"
    .string "run_pipe_test"
    .string "sleep"
    .string "sleep_simple"
    .string "stack_overflow"
    .string "stackful_coroutine"
    .string "stackless_coroutine"
    .string "store_fault"
    .string "sync_sem"
    .string "threads"
    .string "threads_arg"
    .string "until_timeout"
    .string "user_shell"
    .string "usertests"
    .string "usertests_simple"
    .string "yield"

    .section .data
    .global app_0_start
    .global app_0_end
    .align 3
app_0_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/adder"
app_0_end:

    .section .data
    .global app_1_start
    .global app_1_end
    .align 3
app_1_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/adder_atomic"
app_1_end:

    .section .data
    .global app_2_start
    .global app_2_end
    .align 3
app_2_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/adder_mutex_blocking"
app_2_end:

    .section .data
    .global app_3_start
    .global app_3_end
    .align 3
app_3_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/adder_mutex_spin"
app_3_end:

    .section .data
    .global app_4_start
    .global app_4_end
    .align 3
app_4_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/adder_peterson_spin"
app_4_end:

    .section .data
    .global app_5_start
    .global app_5_end
    .align 3
app_5_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/adder_peterson_yield"
app_5_end:

    .section .data
    .global app_6_start
    .global app_6_end
    .align 3
app_6_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/adder_simple_spin"
app_6_end:

    .section .data
    .global app_7_start
    .global app_7_end
    .align 3
app_7_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/adder_simple_yield"
app_7_end:

    .section .data
    .global app_8_start
    .global app_8_end
    .align 3
app_8_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/barrier_condvar"
app_8_end:

    .section .data
    .global app_9_start
    .global app_9_end
    .align 3
app_9_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/barrier_fail"
app_9_end:

    .section .data
    .global app_10_start
    .global app_10_end
    .align 3
app_10_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/cat"
app_10_end:

    .section .data
    .global app_11_start
    .global app_11_end
    .align 3
app_11_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/cmdline_args"
app_11_end:

    .section .data
    .global app_12_start
    .global app_12_end
    .align 3
app_12_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/condsync_condvar"
app_12_end:

    .section .data
    .global app_13_start
    .global app_13_end
    .align 3
app_13_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/condsync_sem"
app_13_end:

    .section .data
    .global app_14_start
    .global app_14_end
    .align 3
app_14_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/count_lines"
app_14_end:

    .section .data
    .global app_15_start
    .global app_15_end
    .align 3
app_15_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/early_exit"
app_15_end:

    .section .data
    .global app_16_start
    .global app_16_end
    .align 3
app_16_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/early_exit2"
app_16_end:

    .section .data
    .global app_17_start
    .global app_17_end
    .align 3
app_17_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/eisenberg"
app_17_end:

    .section .data
    .global app_18_start
    .global app_18_end
    .align 3
app_18_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/exit"
app_18_end:

    .section .data
    .global app_19_start
    .global app_19_end
    .align 3
app_19_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/fantastic_text"
app_19_end:

    .section .data
    .global app_20_start
    .global app_20_end
    .align 3
app_20_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/filetest_simple"
app_20_end:

    .section .data
    .global app_21_start
    .global app_21_end
    .align 3
app_21_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktest"
app_21_end:

    .section .data
    .global app_22_start
    .global app_22_end
    .align 3
app_22_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktest2"
app_22_end:

    .section .data
    .global app_23_start
    .global app_23_end
    .align 3
app_23_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktest_simple"
app_23_end:

    .section .data
    .global app_24_start
    .global app_24_end
    .align 3
app_24_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/forktree"
app_24_end:

    .section .data
    .global app_25_start
    .global app_25_end
    .align 3
app_25_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/hello_world"
app_25_end:

    .section .data
    .global app_26_start
    .global app_26_end
    .align 3
app_26_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/huge_write"
app_26_end:

    .section .data
    .global app_27_start
    .global app_27_end
    .align 3
app_27_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/infloop"
app_27_end:

    .section .data
    .global app_28_start
    .global app_28_end
    .align 3
app_28_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/initproc"
app_28_end:

    .section .data
    .global app_29_start
    .global app_29_end
    .align 3
app_29_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/initproc_test"
app_29_end:

    .section .data
    .global app_30_start
    .global app_30_end
    .align 3
app_30_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/matrix"
app_30_end:

    .section .data
    .global app_31_start
    .global app_31_end
    .align 3
app_31_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/mpsc_sem"
app_31_end:

    .section .data
    .global app_32_start
    .global app_32_end
    .align 3
app_32_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/peterson"
app_32_end:

    .section .data
    .global app_33_start
    .global app_33_end
    .align 3
app_33_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/phil_din_mutex"
app_33_end:

    .section .data
    .global app_34_start
    .global app_34_end
    .align 3
app_34_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/pipe_large_test"
app_34_end:

    .section .data
    .global app_35_start
    .global app_35_end
    .align 3
app_35_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/pipetest"
app_35_end:

    .section .data
    .global app_36_start
    .global app_36_end
    .align 3
app_36_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/priv_csr"
app_36_end:

    .section .data
    .global app_37_start
    .global app_37_end
    .align 3
app_37_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/priv_inst"
app_37_end:

    .section .data
    .global app_38_start
    .global app_38_end
    .align 3
app_38_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/run_pipe_test"
app_38_end:

    .section .data
    .global app_39_start
    .global app_39_end
    .align 3
app_39_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/sleep"
app_39_end:

    .section .data
    .global app_40_start
    .global app_40_end
    .align 3
app_40_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/sleep_simple"
app_40_end:

    .section .data
    .global app_41_start
    .global app_41_end
    .align 3
app_41_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/stack_overflow"
app_41_end:

    .section .data
    .global app_42_start
    .global app_42_end
    .align 3
app_42_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/stackful_coroutine"
app_42_end:

    .section .data
    .global app_43_start
    .global app_43_end
    .align 3
app_43_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/stackless_coroutine"
app_43_end:

    .section .data
    .global app_44_start
    .global app_44_end
    .align 3
app_44_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/store_fault"
app_44_end:

    .section .data
    .global app_45_start
    .global app_45_end
    .align 3
app_45_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/sync_sem"
app_45_end:

    .section .data
    .global app_46_start
    .global app_46_end
    .align 3
app_46_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/threads"
app_46_end:

    .section .data
    .global app_47_start
    .global app_47_end
    .align 3
app_47_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/threads_arg"
app_47_end:

    .section .data
    .global app_48_start
    .global app_48_end
    .align 3
app_48_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/until_timeout"
app_48_end:

    .section .data
    .global app_49_start
    .global app_49_end
    .align 3
app_49_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/user_shell"
app_49_end:

    .section .data
    .global app_50_start
    .global app_50_end
    .align 3
app_50_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/usertests"
app_50_end:

    .section .data
    .global app_51_start
    .global app_51_end
    .align 3
app_51_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/usertests_simple"
app_51_end:

    .section .data
    .global app_52_start
    .global app_52_end
    .align 3
app_52_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/yield"
app_52_end:
