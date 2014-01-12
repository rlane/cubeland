#version 120

const vec4 fog_color = vec4(0.0, 0.75, 1.0, 1.0);

uniform sampler2D texture;

varying vec4 frag_diffuse_factor;
varying vec2 frag_texcoord1;
varying vec2 frag_texcoord2;
varying float frag_tex_factor;
varying float frag_fog_factor;

void main() {
    vec4 noise = mix(texture2D(texture, frag_texcoord1),
                     texture2D(texture, frag_texcoord2),
                     frag_tex_factor);
    gl_FragColor = noise * frag_diffuse_factor;
    gl_FragColor = mix(fog_color, gl_FragColor, frag_fog_factor);
}
