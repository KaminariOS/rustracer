#version 460
#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_ray_tracing : require
#include "lib/RayTracingCommons.glsl"
#include "lib/UniformBufferObject.glsl"

layout(binding = UNIFORM_BIND) readonly uniform UniformBufferObjectStruct { UniformBufferObject Camera; };
layout(binding = DLIGHT_BIND) readonly buffer Lights { Light[] lights; };

layout(binding = SKYBOX_BIND) uniform samplerCube skybox;

layout(location = 0) rayPayloadInEXT RayPayload Ray;

void main()
{
	vec3 light_acc = vec3(0.);
	if (Ray.t != 0) {
		for(int i = 0; i < lights.length(); i++) {
			Light li = lights[i];
			float cos = dot(normalize(li.transform.xyz), normalize(gl_WorldRayDirectionEXT));
			if (cos < 0.) {
				light_acc += -cos * li.color.xyz * li.intensity;
			}
		}
	}
	if (Camera.HasSky)
	{
		// Sky color
		const float t = 0.5 * (normalize(gl_WorldRayDirectionEXT).y + 1);
//		const vec3 skyColor = mix(vec3(1.0), vec3(0.5, 0.7, 1.0), t);
		const vec3 skyColor = texture(skybox, gl_WorldRayDirectionEXT).xyz;
		light_acc += skyColor + light_acc;
	} else
	{
		light_acc += vec3(0.01);
	}
	Ray.hitValue = vec3(0.);
	Ray.needScatter = false;
	Ray.emittance = vec3(light_acc);
	if (lights.length() == 0) {
		Ray.emittance = vec3(0.);
	}
	Ray.t = -1.;
}
