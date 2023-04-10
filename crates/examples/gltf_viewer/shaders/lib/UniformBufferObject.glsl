#include "PunctualLight.glsl"

struct UniformBufferObject
{
	mat4 ModelView;
	mat4 Projection;
	mat4 ModelViewInverse;
	mat4 ProjectionInverse;

	float Aperture;
	float FocusDistance;
	float fovAngle;
	float orthographic_fov_dis;
	float HeatmapScale;
	uint TotalNumberOfSamples;

	uint NumberOfSamples;
	uint NumberOfBounces;
	uint RandomSeed;
	bool HasSky;

	bool antialiasing;
	uint mapping;
	uint frame_count;
	uint debug;

	bool fully_opaque;
	float exposure;
	uint tone_mapping_mode;
};
