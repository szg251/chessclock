#!/bin/sh

set -e

pyocd load --target=STM32F103C8 --format=elf $1
pyocd rtt --target=STM32F103C8 | defmt-print -e $1
