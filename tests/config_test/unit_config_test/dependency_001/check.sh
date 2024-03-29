#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test dependency not exist
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/{conflicts.service,requires.service,wants.service,requisite.service,partof.service,bindsto.service} ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    sctl status base.service &> log
    expect_eq $? 2
    check_log log 'Failed to show the status of base.service: NotExisted' || return 1
    rm -rf log

    # Requires: dependency not exist leads to start failure
    for serv in requires requisite bindsto; do
        sctl start ${serv}
        expect_ne $? 0 || return 1
        check_status ${serv}.service inactive || return 1
    done

    # Wants/Partof: start normally when dependency not exist
    for serv in wants partof; do
        sctl start ${serv}
        expect_eq $? 0 || return 1
        check_status ${serv} active || return 1
    done

    # clean
    sctl stop requires.service wants.service requisite.service partof.service bindsto.service
    kill_sysmaster
}

# usage: test dependency inactive
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1

    # Requires: dependency inactive leads to inactive
    sctl start requires.service
    expect_eq $? 0 || return 1
    check_status requires.service active || return 1
    check_status base.service active || return 1
    sctl stop base.service
    check_status requires.service inactive || return 1

    kill_sysmaster

    # Requires: dependency finish or condition check failed
    sed -i 's/sleep 100/sleep 2/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1
    sctl start requires.service
    expect_eq $? 0 || return 1
    check_status requires.service active || return 1
    check_status base.service active || return 1
    sleep 2
    check_status base.service inactive || return 1
    check_status requires.service active || return 1

    sctl stop requires.service
    kill_sysmaster

    sed -i '/Description/a ConditionPathExists="/notexist"' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1
    sctl start requires.service
    expect_eq $? 0 || return 1
    check_status base.service inactive || return 1
    check_log ${SYSMST_LOG} 'Starting failed because condition test failed'
    check_status requires.service active || return 1

    sctl stop requires.service
    kill_sysmaster

    # Requisite: dependency inactive leads to inactive
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1
    sctl start requisite.service
    expect_eq $? 0 || return 1
    check_status requisite.service inactive || return 1
    check_status base.service inactive || return 1
    sctl start base.service
    check_status base.service active || return 1
    sctl start requisite.service
    expect_eq $? 0 || return 1
    check_status requisite.service active || return 1

    kill_sysmaster

    # Bindsto: dependency inactive
    run_sysmaster || return 1
    sctl start bindsto.service
    expect_eq $? 0 || return 1
    check_status bindsto.service active || return 1
    check_status base.service active || return 1
    sctl stop base.service
    check_status bindsto.service inactive || return 1

    kill_sysmaster

    # Bindsto: dependency finish or condition check failed
    sed -i 's/sleep 100/sleep 2/' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1
    sctl start bindsto.service
    expect_eq $? 0 || return 1
    check_status bindsto.service active || return 1
    check_status base.service active || return 1
    sleep 2
    check_status base.service inactive || return 1
    check_status bindsto.service inactive || return 1

    kill_sysmaster

    sed -i '/Description/a ConditionPathExists="/notexist"' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1
    sctl start bindsto.service
    expect_eq $? 0 || return 1
    check_status base.service inactive || return 1
    check_log ${SYSMST_LOG} 'Starting failed because condition test failed'
    check_status bindsto.service active || return 1

    kill_sysmaster

    sed -i '/Description/a After="base.service"' ${SYSMST_LIB_PATH}/bindsto.service
    run_sysmaster || return 1
    sctl start bindsto.service
    expect_eq $? 0 || return 1
    check_status base.service inactive || return 1
    check_log ${SYSMST_LOG} 'Starting failed because condition test failed'
    check_status bindsto.service inactive || return 1

    kill_sysmaster

    # Wants: stay active when dependency inactive leads to inactive
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    run_sysmaster || return 1
    sctl start wants.service
    expect_eq $? 0 || return 1
    check_status wants.service active || return 1
    check_status base.service active || return 1
    sctl stop base.service
    check_status wants.service active || return 1

    sctl stop wants.service
    kill_sysmaster

    # PartOf: only for dependency stop or restart
    run_sysmaster || return 1
    sctl start partof.service
    expect_eq $? 0 || return 1
    check_status partof.service active || return 1
    check_status base.service inactive || return 1
    sctl start base.service
    check_status base.service active || return 1
    check_status partof.service active || return 1
    base_pid_1="$(get_pids base.service)"
    partof_pid_1="$(get_pids partof.service)"
    sctl restart base.service
    expect_eq $? 0 || return 1
    check_status base.service active || return 1
    check_status partof.service active || return 1
    base_pid_2="$(get_pids base.service)"
    partof_pid_2="$(get_pids partof.service)"
    expect_gt "${base_pid_2}" "${base_pid_1}"
    expect_gt "${partof_pid_2}" "${partof_pid_1}"
    sctl restart partof.service
    expect_eq $? 0 || return 1
    check_status partof.service active || return 1
    check_status base.service active || return 1
    expect_eq "$(get_pids base.service)" "${base_pid_2}"
    expect_gt "$(get_pids partof.service)" "${partof_pid_2}"
    sctl stop base.service
    check_status base.service inactive || return 1
    check_status partof.service inactive || return 1

    sctl stop base.service partof.service

    # clean
    kill_sysmaster
}

# usage: test conflict dependency
function test03() {
    log_info "===== test03 ====="
    run_sysmaster || return 1

    sctl start base.service
    check_status base.service active || return 1

    sctl start conflicts.service
    check_status conflicts.service active || return 1
    check_status base.service inactive || return 1

    sctl start base.service
    check_status base.service active || return 1
    check_status conflicts.service inactive || return 1

    # clean
    sctl stop conflicts.service
    kill_sysmaster
}

# usage: test contradictory dependency
function test04() {
    log_info "===== test04 ====="
    sed -i "/Conflicts/a Requires=\"base.service\"" ${SYSMST_LIB_PATH}/conflicts.service
    run_sysmaster || return 1

    sctl start conflicts.service
    check_status conflicts.service inactive || return 1

    # clean
    kill_sysmaster
}

# usage: test loop dependency
function test05() {
    log_info "===== test05 ====="
    sed -i "/Description/a Requires=\"requires.service\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start requires.service
    check_status requires.service active || return 1
    check_status base.service active || return 1

    # clean
    sctl stop base.service requires.service
    kill_sysmaster
}

# usage: test dependency restart
function test06() {
    log_info "===== test06 ====="
    run_sysmaster || return 1

    # Requires: dependency restart leads to restart
    sctl start requires.service
    expect_eq $? 0 || return 1
    check_status requires.service active || return 1
    check_status base.service active || return 1
    base_pid="$(get_pids base.service)"
    requires_pid="$(get_pids requires.service)"
    sctl restart base.service
    expect_eq $? 0 || return 1
    check_status base.service active || return 1
    check_status requires.service active || return 1
    expect_gt "$(get_pids base.service)" "${base_pid}"
    expect_gt "$(get_pids requires.service)" "${requires_pid}"

    # Wants: stay active when dependency restart
    sctl start wants.service
    expect_eq $? 0 || return 1
    check_status wants.service active || return 1
    check_status base.service active || return 1
    base_pid="$(get_pids base.service)"
    wants_pid="$(get_pids wants.service)"
    sctl restart base.service
    expect_eq $? 0 || return 1
    check_status base.service active || return 1
    check_status wants.service active || return 1
    expect_gt "$(get_pids base.service)" "${base_pid}"
    expect_eq "$(get_pids wants.service)" "${wants_pid}"

    # clean
    sctl stop base.service requires.service wants.service
    kill_sysmaster
}

test01 || exit 1
test02 || exit 1
test03 || exit 1
test04 || exit 1
test05 || exit 1
test06 || exit 1
exit "${EXPECT_FAIL}"
