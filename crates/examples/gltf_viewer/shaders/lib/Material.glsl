#define MIN_DIELECTRICS_F0 0.04f
#define PI 3.141592653589f
#define ONE_OVER_PI (1.0f / PI)
#define TWO_PI (2.0f * PI)


#define evalDiffuse evalFrostbiteDisneyDiffuse
#define diffuseTerm frostbiteDisneyDiffuse

#define sampleSpecular sampleSpecularMicrofacet
#define sampleSpecularHalfVector sampleGGXVNDF
#define Smith_G1 Smith_G1_GGX

#define specularSampleWeight specularSampleWeightGGXVNDF
#define specularPdf sampleGGXVNDFReflectionPdf

// BRDF types
#define DIFFUSE_TYPE 1
#define SPECULAR_TYPE 2
#define TRANSMISSION_TYPE 3

float rsqrt(float x) { return inversesqrt(x); }

struct TextureInfo {
    int index;
    int coord;
};

struct TransmissionInfo {
    TextureInfo transmission_texture;
    float transmission_factor;
    bool exist;
};

struct MetallicRoughnessInfo {
    float metallic_factor;
    float roughness_factor;
    TextureInfo metallic_roughness_texture;
};

struct VolumeInfo {
    vec3 attenuation_color;
    float thickness_factor;
    TextureInfo thickness_texture;
    float attenuation_distance;
    bool exists;
};

struct MaterialRaw {
    uint alpha_mode;
	float alpha_cutoff;
	vec2 _padding;
    float _padding1;
    bool double_sided;

    TextureInfo baseColorTexture;
    vec4 baseColor;

    MetallicRoughnessInfo metallicRoughnessInfo;
    TextureInfo normal_texture;
    TextureInfo emissive_texture;
    vec4 emissive_factor;

    TextureInfo occlusion_texture;
    float ior;
    bool unlit;
    TransmissionInfo transmission_info;
    VolumeInfo volume_info;
};

struct MaterialBrdf {
    vec3 baseColor;
    float metallic;
    float roughness;
    float ior;
    float transmission;
};

// https://github.com/boksajak/referencePT
float shininessToBeckmannAlpha(float shininess) {
	return sqrt(2.0f / (shininess + 2.0f));
}

float saturate(float x) { return clamp(x, 0.0f, 1.0f); }

vec3 baseColorToSpecularF0(vec3 baseColor, float metalness) {
	return mix(vec3(MIN_DIELECTRICS_F0, MIN_DIELECTRICS_F0, MIN_DIELECTRICS_F0), baseColor, metalness);
}

float luminance(vec3 rgb)
{
	return dot(rgb, vec3(0.2126f, 0.7152f, 0.0722f));
}

vec3 baseColorToDiffuseReflectance(vec3 baseColor, float metalness)
{
	return baseColor * (1.0f - metalness);
}

// Schlick's approximation to Fresnel term
// f90 should be 1.0, except for the trick used by Schuler (see 'shadowedF90' function)
vec3 evalFresnelSchlick(vec3 f0, float f90, float NdotS)
{
	return f0 + (f90 - f0) * pow(1.0f - NdotS, 5.0f);
}
struct BRDF {
   float specular;
   float diffuse;
   float transmission;
};


vec3 evalFresnel(vec3 f0, float f90, float NdotS)
{
    // Default is Schlick's approximation
    return evalFresnelSchlick(f0, f90, NdotS);
}


// Attenuates F90 for very low F0 values
// Source: "An efficient and Physically Plausible Real-Time Shading Model" in ShaderX7 by Schuler
// Also see section "Overbright highlights" in Hoffman's 2010 "Crafting Physically Motivated Shading Models for Game Development" for discussion
// IMPORTANT: Note that when F0 is calculated using metalness, it's value is never less than MIN_DIELECTRICS_F0, and therefore,
// this adjustment has no effect. To be effective, F0 must be authored separately, or calculated in different way. See main text for discussion.
float shadowedF90(vec3 F0) {
    // This scaler value is somewhat arbitrary, Schuler used 60 in his article. In here, we derive it from MIN_DIELECTRICS_F0 so
    // that it takes effect for any reflectance lower than least reflective dielectrics
    //const float t = 60.0f;
    const float t = (1.0f / MIN_DIELECTRICS_F0);
    return min(1.0f, t * luminance(F0));
}

BRDF getBrdfProbability(vec3 baseColor, float metalness, vec3 V, vec3 shadingNormal) {

	// Evaluate Fresnel term using the shading normal
	// Note: we use the shading normal instead of the microfacet normal (half-vector) for Fresnel term here. That's suboptimal for rough surfaces at grazing angles, but half-vector is yet unknown at this point
	float specularF0 = luminance(baseColorToSpecularF0(baseColor, metalness));
	float diffuseReflectance = luminance(baseColorToDiffuseReflectance(baseColor, metalness));
	float Fresnel = saturate(luminance(evalFresnel(vec3(specularF0), shadowedF90(vec3(specularF0)), max(0.0f, dot(V, shadingNormal)))));

	// Approximate relative contribution of BRDFs using the Fresnel term
	float specular = Fresnel;
	float diffuse = diffuseReflectance * (1.0f - Fresnel); //< If diffuse term is weighted by Fresnel, apply it here as well

	// Return probability of selecting specular BRDF over diffuse BRDF
	float p = (specular / max(0.0001f, (specular + diffuse)));
    p = clamp(p, 0.1f, 0.9f);
    BRDF brdf;
    brdf.specular = p;
    brdf.diffuse = 1 - p;
    brdf.transmission = 0;
	// Clamp probability to avoid undersampling of less prominent BRDF
	return brdf;
}

// Calculates rotation quaternion from input vector to the vector (0, 0, 1)
// Input vector must be normalized!
vec4 getRotationToZAxis(vec3 input_vec) {

// Handle special case when input is exact or near opposite of (0, 0, 1)
if (input_vec.z < -0.99999f) return vec4(1.0f, 0.0f, 0.0f, 0.0f);

return normalize(vec4(input_vec.y, -input_vec.x, 0.0f, 1.0f + input_vec.z));
}


// Optimized point rotation using quaternion
// Source: https://gamedev.stackexchange.com/questions/28395/rotating-vector3-by-a-quaternion
vec3 rotatePoint(vec4 q, vec3 v) {
    const vec3 qAxis = vec3(q.x, q.y, q.z);
    return 2.0f * dot(qAxis, v) * qAxis + (q.w * q.w - dot(qAxis, qAxis)) * v + 2.0f * q.w * cross(qAxis, v);
}


// Samples a direction within a hemisphere oriented along +Z axis with a cosine-weighted distribution
// Source: "Sampling Transformations Zoo" in Ray Tracing Gems by Shirley et al.
vec3 sampleHemisphere(vec2 u, inout float pdf) {

float a = sqrt(u.x);
float b = TWO_PI * u.y;

vec3 result = vec3(
a * cos(b),
a * sin(b),
sqrt(1.0f - u.x));

pdf = result.z * ONE_OVER_PI;

return result;
}

vec3 sampleHemisphere(vec2 u) {
    float pdf;
    return sampleHemisphere(u, pdf);
}

// Data needed to evaluate BRDF (surface and material properties at given point + configuration of light and normal vectors)
struct BrdfData
{
// Material properties
    vec3 specularF0;
    vec3 diffuseReflectance;

// Roughnesses
    float roughness;    //< perceptively linear roughness (artist's input)
    float alpha;        //< linear roughness - often 'alpha' in specular BRDF equations
    float alphaSquared; //< alpha squared - pre-calculated value commonly used in BRDF equations

// Commonly used terms for BRDF evaluation
    vec3 F; //< Fresnel term

// Vectors
    vec3 V; //< Direction to viewer (or opposite direction of incident ray)
    vec3 N; //< Shading normal
    vec3 H; //< Half vector (microfacet normal)
    vec3 L; //< Direction to light (or direction of reflecting ray)

    float NdotL;
    float NdotV;

    float LdotH;
    float NdotH;
    float VdotH;

// True when V/L is backfacing wrt. shading normal N
    bool Vbackfacing;
    bool Lbackfacing;
};

float GGX_D(float alphaSquared, float NdotH) {
    float b = ((alphaSquared - 1.0f) * NdotH * NdotH + 1.0f);
    return alphaSquared / (PI * b * b);
}

// -------------------------------------------------------------------------
//    Microfacet model
// -------------------------------------------------------------------------

// Samples a microfacet normal for the GGX distribution using VNDF method.
// Source: "Sampling the GGX Distribution of Visible Normals" by Heitz
// See also https://hal.inria.fr/hal-00996995v1/document and http://jcgt.org/published/0007/04/01/
// Random variables 'u' must be in <0;1) interval
// PDF is 'G1(NdotV) * D'
vec3 sampleGGXVNDF(vec3 Ve, vec2 alpha2D, vec2 u) {

    // Section 3.2: transforming the view direction to the hemisphere configuration
    vec3 Vh = normalize(vec3(alpha2D.x * Ve.x, alpha2D.y * Ve.y, Ve.z));

    // Section 4.1: orthonormal basis (with special case if cross product is zero)
    float lensq = Vh.x * Vh.x + Vh.y * Vh.y;
    vec3 T1 = lensq > 0.0f ? vec3(-Vh.y, Vh.x, 0.0f) * rsqrt(lensq) : vec3(1.0f, 0.0f, 0.0f);
    vec3 T2 = cross(Vh, T1);

    // Section 4.2: parameterization of the projected area
    float r = sqrt(u.x);
    float phi = TWO_PI * u.y;
    float t1 = r * cos(phi);
    float t2 = r * sin(phi);
    float s = 0.5f * (1.0f + Vh.z);
    t2 = mix(sqrt(1.0f - t1 * t1), t2, s);

    // Section 4.3: reprojection onto hemisphere
    vec3 Nh = t1 * T1 + t2 * T2 + sqrt(max(0.0f, 1.0f - t1 * t1 - t2 * t2)) * Vh;

    // Section 3.4: transforming the normal back to the ellipsoid configuration
    return normalize(vec3(alpha2D.x * Nh.x, alpha2D.y * Nh.y, max(0.0f, Nh.z)));
}


// Smith G1 term (masking function) optimized for GGX distribution (by substituting G_Lambda_GGX into G1)
float Smith_G1_GGX(float a) {
    float a2 = a * a;
    return 2.0f / (sqrt((a2 + 1.0f) / a2) + 1.0f);
}

// Smith G1 term (masking function) further optimized for GGX distribution (by substituting G_a into G1_GGX)
float Smith_G1_GGX(float alpha, float NdotS, float alphaSquared, float NdotSSquared) {
    return 2.0f / (sqrt(((alphaSquared * (1.0f - NdotSSquared)) + NdotSSquared) / NdotSSquared) + 1.0f);
}

// PDF of sampling a reflection vector L using 'sampleGGXVNDF'.
// Note that PDF of sampling given microfacet normal is (G1 * D) when vectors are in local space (in the hemisphere around shading normal).
// Remaining terms (1.0f / (4.0f * NdotV)) are specific for reflection case, and come from multiplying PDF by jacobian of reflection operator
float sampleGGXVNDFReflectionPdf(float alpha, float alphaSquared, float NdotH, float NdotV, float LdotH) {
    NdotH = max(0.00001f, NdotH);
    NdotV = max(0.00001f, NdotV);
    return (GGX_D(max(0.00001f, alphaSquared), NdotH) * Smith_G1_GGX(alpha, NdotV, alphaSquared, NdotV * NdotV)) / (4.0f * NdotV);
}

// Weight for the reflection ray sampled from GGX distribution using VNDF method
float specularSampleWeightGGXVNDF(float alpha, float alphaSquared, float NdotL, float NdotV, float HdotL, float NdotH) {
    //    #if USE_HEIGHT_CORRELATED_G2
    //    return Smith_G2_Over_G1_Height_Correlated(alpha, alphaSquared, NdotL, NdotV);
    //    #else
    return Smith_G1_GGX(alpha, NdotL, alphaSquared, NdotL * NdotL);
    //    #endif
}
// Frostbite's version of Disney diffuse with energy normalization.
// Source: "Moving Frostbite to Physically Based Rendering" by Lagarde & de Rousiers
float frostbiteDisneyDiffuse(const BrdfData data) {
    float energyBias = 0.5f * data.roughness;
    float energyFactor = mix(1.0f, 1.0f / 1.51f, data.roughness);

    float FD90MinusOne = energyBias + 2.0 * data.LdotH * data.LdotH * data.roughness - 1.0f;

    float FDL = 1.0f + (FD90MinusOne * pow(1.0f - data.NdotL, 5.0f));
    float FDV = 1.0f + (FD90MinusOne * pow(1.0f - data.NdotV, 5.0f));

    return FDL * FDV * energyFactor;
}

vec3 evalFrostbiteDisneyDiffuse(const BrdfData data) {
    return data.diffuseReflectance * (frostbiteDisneyDiffuse(data) * ONE_OVER_PI * data.NdotL);
}

// -------------------------------------------------------------------------
//    Combined BRDF
// -------------------------------------------------------------------------

// Precalculates commonly used terms in BRDF evaluation
// Clamps around dot products prevent NaNs and ensure numerical stability, but make sure to
// correctly ignore rays outside of the sampling hemisphere, by using 'Vbackfacing' and 'Lbackfacing' flags
BrdfData prepareBRDFData(vec3 N, vec3 L, vec3 V, MaterialBrdf material) {
    BrdfData data;

    // Evaluate VNHL vectors
    data.V = V;
    data.N = N;
    data.H = normalize(L + V);
    data.L = L;

    float NdotL = dot(N, L);
    float NdotV = dot(N, V);
    data.Vbackfacing = (NdotV <= 0.0f);
    data.Lbackfacing = (NdotL <= 0.0f);

    // Clamp NdotS to prevent numerical instability. Assume vectors below the hemisphere will be filtered using 'Vbackfacing' and 'Lbackfacing' flags
    data.NdotL = min(max(0.00001f, NdotL), 1.0f);
    data.NdotV = min(max(0.00001f, NdotV), 1.0f);

    data.LdotH = saturate(dot(L, data.H));
    data.NdotH = saturate(dot(N, data.H));
    data.VdotH = saturate(dot(V, data.H));

    // Unpack material properties
    data.specularF0 = baseColorToSpecularF0(material.baseColor, material.metallic);
    data.diffuseReflectance = baseColorToDiffuseReflectance(material.baseColor, material.metallic);

    // Unpack 'perceptively linear' -> 'linear' -> 'squared' roughness
    data.roughness = material.roughness;
    data.alpha = material.roughness * material.roughness;
    data.alphaSquared = data.alpha * data.alpha;

    // Pre-calculate some more BRDF terms
    data.F = evalFresnel(data.specularF0, shadowedF90(data.specularF0), data.LdotH);

    return data;
}


// Returns the quaternion with inverted rotation
vec4 invertRotation(vec4 q)
{
    return vec4(-q.x, -q.y, -q.z, q.w);
}

// Samples a reflection ray from the rough surface using selected microfacet distribution and sampling method
// Resulting weight includes multiplication by cosine (NdotL) term
vec3 sampleSpecularMicrofacet(vec3 Vlocal, float alpha, float alphaSquared, vec3 specularF0, vec2 u, inout vec3 weight) {

// Sample a microfacet normal (H) in local space
    vec3 Hlocal;
    if (alpha == 0.0f) {
        // Fast path for zero roughness (perfect reflection), also prevents NaNs appearing due to divisions by zeroes
        Hlocal = vec3(0.0f, 0.0f, 1.0f);
    } else {
        // For non-zero roughness, this calls VNDF sampling for GG-X distribution or Walter's sampling for Beckmann distribution
        Hlocal = sampleSpecularHalfVector(Vlocal, vec2(alpha, alpha), u);
    }

    // Reflect view direction to obtain light vector
    vec3 Llocal = reflect(-Vlocal, Hlocal);

    // Note: HdotL is same as HdotV here
    // Clamp dot products here to small value to prevent numerical instability. Assume that rays incident from below the hemisphere have been filtered
    float HdotL = max(0.00001f, min(1.0f, dot(Hlocal, Llocal)));
    const vec3 Nlocal = vec3(0.0f, 0.0f, 1.0f);
    float NdotL = max(0.00001f, min(1.0f, dot(Nlocal, Llocal)));
    float NdotV = max(0.00001f, min(1.0f, dot(Nlocal, Vlocal)));
    float NdotH = max(0.00001f, min(1.0f, dot(Nlocal, Hlocal)));
    vec3 F = evalFresnel(specularF0, shadowedF90(specularF0), HdotL);

    // Calculate weight of the sample specific for selected sampling method
    // (this is microfacet BRDF divided by PDF of sampling method - notice how most terms cancel out)
    weight = F * specularSampleWeight(alpha, alphaSquared, NdotL, NdotV, HdotL, NdotH);

    return Llocal;
}

bool evalIndirectCombinedBRDF(vec2 u, vec3 shadingNormal, vec3 geometryNormal, vec3 V, MaterialBrdf material, const uint brdfType, inout vec3 rayDirection, inout vec3 sampleWeight) {
    if (dot(shadingNormal, V) <= 0.0f) return false;
    vec4 qRotationToZ = getRotationToZAxis(shadingNormal);
    vec3 Vlocal = rotatePoint(qRotationToZ, V);
    const vec3 Nlocal = vec3(0.0f, 0.0f, 1.0f);

    vec3 rayDirectionLocal = vec3(0.0f, 0.0f, 0.0f);
    if (brdfType == DIFFUSE_TYPE) {
        rayDirectionLocal = sampleHemisphere(u);
        const BrdfData data = prepareBRDFData(Nlocal, rayDirectionLocal, Vlocal, material);

        // Function 'diffuseTerm' is predivided by PDF of sampling the cosine weighted hemisphere
        sampleWeight = data.diffuseReflectance * diffuseTerm(data);


        // Sample a half-vector of specular BRDF. Note that we're reusing random variable 'u' here, but correctly it should be an new independent random number
        vec3 Hspecular = sampleSpecularHalfVector(Vlocal, vec2(data.alpha, data.alpha), u);

        // Clamp HdotL to small value to prevent numerical instability. Assume that rays incident from below the hemisphere have been filtered
        float VdotH = max(0.00001f, min(1.0f, dot(Vlocal, Hspecular)));
        sampleWeight *= (vec3(1.0f, 1.0f, 1.0f) - evalFresnel(data.specularF0, shadowedF90(data.specularF0), VdotH));
    }
    else if (brdfType == SPECULAR_TYPE) {
        const BrdfData data = prepareBRDFData(Nlocal, vec3(0.0f, 0.0f, 1.0f) /* unused L vector */, Vlocal, material);
        rayDirectionLocal = sampleSpecular(Vlocal, data.alpha, data.alphaSquared, data.specularF0, u, sampleWeight);
    }

    // Prevent tracing direction with no contribution
    if (luminance(sampleWeight) == 0.0f) return false;

    // Transform sampled direction Llocal back to V vector space
    rayDirection = normalize(rotatePoint(invertRotation(qRotationToZ), rayDirectionLocal));

    // Prevent tracing direction "under" the hemisphere (behind the triangle)
    if (dot(geometryNormal, rayDirection) <= 0.0f) return false;
    return true;
    }
