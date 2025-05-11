#!/usr/bin/env python3

import numpy as np
import tifffile as tf
import cv2
from sys import argv
from hexdump import hexdump


if len(argv) < 2: 
    print("{} [raw file]".format(argv[0]))

with open(argv[1], "rb") as f:
    data = f.read()

# 8-bit RGGB (Bayer pattern)
arr = np.frombuffer(data, dtype=np.dtype(np.uint8))
arr.shape = (1740, 2320, 1)
#arr.shape = (3488, 4632, 1)
print(arr)

#colimg = cv2.cvtColor(arr, cv2.COLOR_BAYER_BGGR2RGB)
colimg = cv2.cvtColor(arr, cv2.COLOR_BAYER_RGGB2RGB)
#colimg = cv2.cvtColor(arr, cv2.COLOR_BAYER_GBRG2RGB)
#colimg = cv2.cvtColor(arr, cv2.COLOR_BAYER_GRBG2RGB)

with open("/tmp/wow.rgb8.raw", "wb") as f:
    f.write(colimg.data)

cv2.imwrite("/tmp/wow.png", colimg)
