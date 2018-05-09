// BEGINNING OF INCLUDE BLOCK V1

float level(float irrad, float white_level) {
  if ( (white_level < 1.0) && (irrad >= 65504 * white_level) ) {
    return 65504; // max fp16 value (don't wrap negative!)
  }
  return irrad / white_level;
}

vec3 level3(vec3 irrad, float white_level) {
  return vec3(
    level(irrad.r, white_level),
    level(irrad.g, white_level),
    level(irrad.b, white_level));
}

vec3 improved_blinn_phong(
  vec3 normal, vec3 viewdir, vec3 lightdir, vec3 light_irradiance,
  vec3 kdiff, vec3 kspec, float shininess)
{
  float cos = max(dot(normal, lightdir), 0);
  vec3 halfdir = normalize(lightdir + viewdir);
  float coshalf = max(dot(normal, halfdir), 0);
  return (kdiff + kspec * pow(coshalf, shininess)) * light_irradiance * cos;
}

// END OF INCLUDE BLOCK V1
