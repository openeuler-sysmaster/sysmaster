#!/bin/bash
p_w_d=`pwd`
echo $p_w_d

target_dir=$1
echo $target_dir
cp -a $p_w_d/test_units/*  $target_dir
exit $?