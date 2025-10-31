#!/bin/bash

rm g210x || true
aarch64-linux-gnu-g++ -og210x g210x.cpp gpiointerface.cpp -lpigpio
