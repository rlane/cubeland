#version 120

const vec4 fog_color = vec4(0.0, 0.75, 1.0, 1.0);
const vec4 grass_color = vec4(0.0, 1.0, 0.0, 1.0);

uniform sampler2D texture;

varying vec4 frag_diffuse_factor;
varying vec2 frag_texcoord;
varying float frag_fog_factor;

void main() {
    gl_FragColor = texture2D(texture, frag_texcoord) * grass_color * frag_diffuse_factor;
    gl_FragColor = mix(fog_color, gl_FragColor, frag_fog_factor);
}
