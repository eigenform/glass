#version 330

/*
From http://jgt.akpeters.com/papers/McGuire08/

Efficient, High-Quality Bayer Demosaic Filtering on GPUs

Morgan McGuire

This paper appears in issue Volume 13, Number 4.
---------------------------------------------------------
Copyright (c) 2008, Morgan McGuire. All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.

    * Redistributions in binary form must reproduce the above
      copyright notice, this list of conditions and the following
      disclaimer in the documentation and/or other materials provided
      with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/

//Vertex Shader


// (w,h,1/w,1/h)
uniform vec4        sourceSize;

// Pixel position of the first red pixel in the Bayer pattern.
// [{0,1}, {0,1}]
uniform vec2        firstRed;

// .xy = Pixel being sampled in the fragment shader on the range [0, 1]
// .zw = ...on the range [0, sourceSize], offset by firstRed 
out vec4            center;

// out_center.x + (-2/w, -1/w, 1/w, 2/w)
// These are the x-positions of the adjacent pixels.
out vec4            xCoord;

// out_center.y + (-2/h, -1/h, 1/h, 2/h); 
// These are the y-positions of the adjacent pixels.
out vec4            yCoord;

in vec2 position;
in vec2 tex_coords;

void main(void) {
    center.xy = tex_coords;
    center.zw = tex_coords * sourceSize.xy + firstRed;

    vec2 invSize = sourceSize.zw;
	vec4 offsets = vec4(
		-2.0 * invSize.x, -invSize.x, invSize.x, 2.0 * invSize.x
	);
    xCoord = center.x + offsets;
    yCoord = center.y + offsets;

    gl_Position = vec4(position, 0.0, 1.0);
}


