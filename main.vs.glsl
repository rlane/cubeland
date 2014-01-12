#version 110

uniform mat4 modelview;
uniform mat4 projection;

attribute vec3 position;
attribute vec3 normal;

varying vec4 frag_diffuse_factor;
varying vec2 frag_texcoord;
varying float frag_fog_factor;

const vec3 light_direction = vec3(0.408248, -0.816497, 0.408248);
const vec4 light_diffuse = vec4(0.8, 0.8, 0.8, 0.0);
const vec4 light_ambient = vec4(0.2, 0.2, 0.2, 1.0);

const float planet_radius = 6371000.0 / 5000.0;
const float fog_density = 0.003;

void main() {
    vec4 eye_position = modelview * vec4(position, 1.0);

    /* Curvature of the planet */
    float distance_squared = pow(eye_position.x, 2.0) + pow(eye_position.z, 2.0);
    eye_position.y -= planet_radius - sqrt(pow(planet_radius, 2.0) - distance_squared);

    gl_Position = projection * eye_position;

    vec4 diffuse_factor
        = max(-dot(normal, light_direction), 0.0) * light_diffuse;
    frag_diffuse_factor = diffuse_factor + light_ambient;

    frag_fog_factor = clamp(exp2(-pow(length(eye_position), 2.0) * pow(fog_density, 2.0) * 1.44), 0.0, 1.0);

    if (normal.x != 0.0) {
        frag_texcoord = position.yz;
    } else if (normal.y != 0.0) {
        frag_texcoord = position.xz;
    } else {
        frag_texcoord = position.xy;
    }
    frag_texcoord *= 16.0/128.0;
}
