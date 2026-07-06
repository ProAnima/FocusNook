import { useEffect, useRef, type CSSProperties } from "react";
import { getLiveThemeShader } from "../shared/themeCatalog";
import type { LiveThemeShaderConfig } from "../shared/themeCatalog";
import type { ResolvedTheme } from "../shared/theme-context";

const VERTEX_SHADER_SOURCE = `
attribute vec2 a_position;

void main() {
  gl_Position = vec4(a_position, 0.0, 1.0);
}
`;

const FRAGMENT_SHADER_SOURCE = `
precision mediump float;

uniform vec2 u_resolution;
uniform float u_time;
uniform float u_intensity;
uniform vec3 u_color_a;
uniform vec3 u_color_b;
uniform vec3 u_color_c;

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);

  return mix(
    mix(hash(i + vec2(0.0, 0.0)), hash(i + vec2(1.0, 0.0)), u.x),
    mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x),
    u.y
  );
}

void main() {
  vec2 rawUv = gl_FragCoord.xy / u_resolution.xy;
  vec2 uv = rawUv;
  uv.x *= u_resolution.x / u_resolution.y;

  float drift = u_time * 0.055;
  float n1 = noise(uv * 2.1 + vec2(drift, -drift * 0.72));
  float n2 = noise(uv * 4.0 + vec2(-drift * 0.55, drift * 0.82));
  float wave = sin((uv.x + uv.y) * 5.4 + drift * 5.2 + n1 * 2.4) * 0.5 + 0.5;
  float sweep = smoothstep(0.15, 0.92, n1 * 0.74 + wave * n2 * 0.46);

  vec3 color = mix(u_color_a, u_color_b, sweep);
  color = mix(color, u_color_c, smoothstep(0.38, 0.96, wave * 0.72 + n2 * 0.36));

  float vignette = smoothstep(0.82, 0.18, distance(rawUv, vec2(0.5, 0.52)));
  float veil = 0.56 + vignette * 0.42;
  gl_FragColor = vec4(color * u_intensity * veil, 1.0);
}
`;

function compileShader(gl: WebGLRenderingContext, type: number, source: string): WebGLShader | null {
  const shader = gl.createShader(type);
  if (!shader) return null;
  gl.shaderSource(shader, source);
  gl.compileShader(shader);

  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    gl.deleteShader(shader);
    return null;
  }

  return shader;
}

function createProgram(gl: WebGLRenderingContext): WebGLProgram | null {
  const vertexShader = compileShader(gl, gl.VERTEX_SHADER, VERTEX_SHADER_SOURCE);
  const fragmentShader = compileShader(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER_SOURCE);
  if (!vertexShader || !fragmentShader) return null;

  const program = gl.createProgram();
  if (!program) return null;

  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);
  gl.deleteShader(vertexShader);
  gl.deleteShader(fragmentShader);

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    gl.deleteProgram(program);
    return null;
  }

  return program;
}

function hexToRgb(hex: string): [number, number, number] {
  const normalized = hex.replace("#", "");
  const value = Number.parseInt(normalized, 16);
  return [
    ((value >> 16) & 255) / 255,
    ((value >> 8) & 255) / 255,
    (value & 255) / 255,
  ];
}

type LiveBackgroundStyle = CSSProperties & {
  "--live-bg-a": string;
  "--live-bg-b": string;
  "--live-bg-c": string;
};

interface ShaderLocations {
  position: number;
  resolution: WebGLUniformLocation | null;
  time: WebGLUniformLocation | null;
  intensity: WebGLUniformLocation | null;
  colorA: WebGLUniformLocation | null;
  colorB: WebGLUniformLocation | null;
  colorC: WebGLUniformLocation | null;
}

function getWebGlContext(canvas: HTMLCanvasElement): WebGLRenderingContext | null {
  const options = { antialias: false, alpha: true, premultipliedAlpha: false };
  try {
    const webgl = canvas.getContext("webgl", options) as WebGLRenderingContext | null;
    return webgl ?? (canvas.getContext("experimental-webgl", options) as WebGLRenderingContext | null);
  } catch {
    return null;
  }
}

function getShaderLocations(gl: WebGLRenderingContext, program: WebGLProgram): ShaderLocations {
  return {
    position: gl.getAttribLocation(program, "a_position"),
    resolution: gl.getUniformLocation(program, "u_resolution"),
    time: gl.getUniformLocation(program, "u_time"),
    intensity: gl.getUniformLocation(program, "u_intensity"),
    colorA: gl.getUniformLocation(program, "u_color_a"),
    colorB: gl.getUniformLocation(program, "u_color_b"),
    colorC: gl.getUniformLocation(program, "u_color_c"),
  };
}

function bindFullscreenQuad(gl: WebGLRenderingContext, positionLocation: number) {
  const buffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
    gl.STATIC_DRAW,
  );
  gl.enableVertexAttribArray(positionLocation);
  gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);
  return buffer;
}

function resizeCanvas(
  gl: WebGLRenderingContext,
  canvas: HTMLCanvasElement,
  resolutionLocation: WebGLUniformLocation | null,
) {
  const pixelRatio = Math.min(window.devicePixelRatio || 1, 2);
  const width = Math.max(1, Math.floor(canvas.clientWidth * pixelRatio));
  const height = Math.max(1, Math.floor(canvas.clientHeight * pixelRatio));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }
  gl.viewport(0, 0, width, height);
  gl.uniform2f(resolutionLocation, width, height);
}

function applyThemeUniforms(
  gl: WebGLRenderingContext,
  locations: ShaderLocations,
  config: LiveThemeShaderConfig,
) {
  const [colorA, colorB, colorC] = config.colors.map(hexToRgb);
  gl.uniform3fv(locations.colorA, colorA);
  gl.uniform3fv(locations.colorB, colorB);
  gl.uniform3fv(locations.colorC, colorC);
  gl.uniform1f(locations.intensity, config.intensity);
}

function mountShader(canvas: HTMLCanvasElement, config: LiveThemeShaderConfig) {
  const gl = getWebGlContext(canvas);
  if (!gl) return undefined;
  const program = createProgram(gl);
  if (!program) return undefined;

  gl.useProgram(program);
  const locations = getShaderLocations(gl, program);
  const buffer = bindFullscreenQuad(gl, locations.position);
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  let animationFrame: number | null = null;

  applyThemeUniforms(gl, locations, config);
  const draw = (time: number) => {
    resizeCanvas(gl, canvas, locations.resolution);
    gl.uniform1f(locations.time, time * 0.001 * config.speed);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    if (!reducedMotion) animationFrame = requestAnimationFrame(draw);
  };
  const resizeObserver = new ResizeObserver(() => resizeCanvas(gl, canvas, locations.resolution));
  resizeObserver.observe(canvas);
  draw(0);

  return () => {
    resizeObserver.disconnect();
    if (animationFrame !== null) cancelAnimationFrame(animationFrame);
    if (buffer) gl.deleteBuffer(buffer);
    gl.deleteProgram(program);
  };
}

export function LiveBackground({ theme }: { theme: ResolvedTheme }) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const config = getLiveThemeShader(theme);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !config) return undefined;
    return mountShader(canvas, config);
  }, [config]);

  if (!config) return null;

  const [a, b, c] = config.colors;
  const style: LiveBackgroundStyle = {
    "--live-bg-a": a,
    "--live-bg-b": b,
    "--live-bg-c": c,
  };

  return (
    <div className="live-background" style={style} aria-hidden="true">
      <canvas ref={canvasRef} className="live-background-canvas" />
      <span className="live-background-fallback" />
    </div>
  );
}
