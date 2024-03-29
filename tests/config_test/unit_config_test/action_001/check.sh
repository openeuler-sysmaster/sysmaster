#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test SuccessAction
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Description/ a SuccessAction="none"' ${SYSMST_LIB_PATH}/base.service
    sed -i 's/sleep 100/sleep 2/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    check_status base active || return 1
    check_status base inactive || return 1
    ps aux | grep -v grep | awk '{print $2}' | grep -w "${sysmaster_pid}"
    expect_eq $? 0 || ps -elf

    # clean
    kill_sysmaster

    # service success: reboot, poweroff, exit
    local cmd_list="reboot poweroff exit"
    for cmd in ${cmd_list}; do
        cp -arf "${work_dir}"/tmp_units/"${cmd}".target ${SYSMST_LIB_PATH} || return 1
        sed -i "s/SuccessAction=.*/SuccessAction=\"${cmd}\"/" ${SYSMST_LIB_PATH}/base.service
        run_sysmaster || return 1

        sctl start base
        check_status base active || return 1
        check_status base inactive || return 1
        ps aux | grep -v grep | awk '{print $2}' | grep -w "${sysmaster_pid}"
        expect_eq $? 0 || ps -elf
        check_log "${SYSMST_LOG}" "by starting ${cmd}.target caused by unit base.service succeeded" || return 1

        # clean
        kill_sysmaster
    done

    # force/immediate reboot/poweroff/exit config in docker leads to exit
    if ! ps aux | head -n2 | grep 'check.log'; then
        # service success: reboot-force, poweroff-force, exit-force
        for cmd in ${cmd_list}; do
            cp -arf "${work_dir}"/tmp_units/"${cmd}".target ${SYSMST_LIB_PATH} || return 1
            sed -i "s/SuccessAction=.*/SuccessAction=\"${cmd}-force\"/" ${SYSMST_LIB_PATH}/base.service
            run_sysmaster || return 1

            sctl start base
            check_status base active || return 1
            check_status base inactive || return 1
            ps aux | grep -v grep | awk '{print $2}' | grep -w "${sysmaster_pid}"
            expect_eq $? 0 || ps -elf
            check_log "${SYSMST_LOG}" "by starting ${cmd}.target caused by unit base.service succeeded" || return 1

            # clean
            kill_sysmaster
        done

        # service success: reboot-immediate, poweroff-immediate, exit-immediate
        for cmd in ${cmd_list}; do
            cp -arf "${work_dir}"/tmp_units/"${cmd}".target ${SYSMST_LIB_PATH} || return 1
            sed -i "s/SuccessAction=.*/SuccessAction=\"${cmd}-immediate\"/" ${SYSMST_LIB_PATH}/base.service
            run_sysmaster || return 1

            sctl start base
            check_status base active || return 1
            check_status base inactive || return 1
            ps aux | grep -v grep | awk '{print $2}' | grep -w "${sysmaster_pid}"
            expect_eq $? 0 || ps -elf
            check_log "${SYSMST_LOG}" "by starting ${cmd}.target caused by unit base.service succeeded" || return 1

            # clean
            kill_sysmaster
        done
    fi

    # service fail: reboot, poweroff, exit
    sed -i 's/sleep.*/false"/' ${SYSMST_LIB_PATH}/base.service
    for cmd in ${cmd_list}; do
        sed -i "s/SuccessAction=.*/SuccessAction=\"${cmd}\"/" ${SYSMST_LIB_PATH}/base.service
        run_sysmaster || return 1

        sctl start base
        check_status base failed || return 1
        ps aux | grep -v grep | awk '{print $2}' | grep -w "${sysmaster_pid}"
        expect_eq $? 0 || ps -elf
        grep -a "by starting .*target caused by unit base.service" "${SYSMST_LOG}"
        expect_eq $? 1 || cat "${SYSMST_LOG}"

        # clean
        kill_sysmaster
    done
}

# usage: test FailureAction
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i '/Description/ a FailureAction="none"' ${SYSMST_LIB_PATH}/base.service
    sed -i 's/sleep 100/sleep 2/' ${SYSMST_LIB_PATH}/base.service

    # service success: reboot, poweroff, exit
    local cmd_list="reboot poweroff exit"
    for cmd in ${cmd_list}; do
        cp -arf "${work_dir}"/tmp_units/"${cmd}".target ${SYSMST_LIB_PATH} || return 1
        sed -i "s/FailureAction=.*/FailureAction=\"${cmd}\"/" ${SYSMST_LIB_PATH}/base.service
        run_sysmaster || return 1

        sctl start base
        check_status base active || return 1
        check_status base inactive || return 1
        ps aux | grep -v grep | awk '{print $2}' | grep -w "${sysmaster_pid}"
        expect_eq $? 0 || ps -elf
        grep -a "by starting .*target caused by unit base.service" "${SYSMST_LOG}"
        expect_eq $? 1 || cat "${SYSMST_LOG}"

        # clean
        kill_sysmaster
    done

    # service fail: reboot, poweroff, exit
    sed -i 's/sleep.*/false"/' ${SYSMST_LIB_PATH}/base.service
    for cmd in ${cmd_list}; do
        sed -i "s/FailureAction=.*/FailureAction=\"${cmd}\"/" ${SYSMST_LIB_PATH}/base.service
        run_sysmaster || return 1

        sctl start base
        check_status base failed || return 1
        ps aux | grep -v grep | awk '{print $2}' | grep -w "${sysmaster_pid}"
        expect_eq $? 0 || ps -elf
        check_log "${SYSMST_LOG}" "by starting ${cmd}.target caused by unit base.service failed" || return 1

        # clean
        kill_sysmaster
    done

    # force/immediate reboot/poweroff/exit config in docker leads to exit
    ps aux | head -n2 | grep 'check.log' && return

    # service fail: reboot-force, poweroff-force, exit-force
    sed -i 's/sleep.*/false"/' ${SYSMST_LIB_PATH}/base.service
    for cmd in ${cmd_list}; do
        sed -i "s/FailureAction=.*/FailureAction=\"${cmd}\"-force/" ${SYSMST_LIB_PATH}/base.service
        run_sysmaster || return 1

        sctl start base
        check_status base failed || return 1
        ps aux | grep -v grep | awk '{print $2}' | grep -w "${sysmaster_pid}"
        expect_eq $? 0 || ps -elf
        check_log "${SYSMST_LOG}" "by starting ${cmd}.target caused by unit base.service failed" || return 1

        # clean
        kill_sysmaster
    done

    # service fail: reboot-immediate, poweroff-immediate, exit-immediate
    sed -i 's/sleep.*/false"/' ${SYSMST_LIB_PATH}/base.service
    for cmd in ${cmd_list}; do
        sed -i "s/FailureAction=.*/FailureAction=\"${cmd}\"-immediate/" ${SYSMST_LIB_PATH}/base.service
        run_sysmaster || return 1

        sctl start base
        check_status base failed || return 1
        ps aux | grep -v grep | awk '{print $2}' | grep -w "${sysmaster_pid}"
        expect_eq $? 0 || ps -elf
        check_log "${SYSMST_LOG}" "by starting ${cmd}.target caused by unit base.service failed" || return 1

        # clean
        kill_sysmaster
    done
}

test01 || exit 1
test02 || exit 1
exit "${EXPECT_FAIL}"
