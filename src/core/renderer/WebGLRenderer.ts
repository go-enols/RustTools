/**
 * 高性能 WebGL 渲染器
 * 
 * 用于优化桌面捕获和视频帧的渲染性能
 */

export class WebGLRenderer {
  private canvas: HTMLCanvasElement;
  private gl: WebGLRenderingContext | WebGL2RenderingContext;
  private program: WebGLProgram | null = null;
  private texture: WebGLTexture | null = null;
  private positionBuffer: WebGLBuffer | null = null;
  
  // 顶点着色器
  private static vertexShaderSource = `
    attribute vec2 a_position;
    attribute vec2 a_texCoord;
    varying vec2 v_texCoord;
    
    void main() {
      gl_Position = vec4(a_position, 0.0, 1.0);
      v_texCoord = a_texCoord;
    }
  `;
  
  // 片段着色器
  private static fragmentShaderSource = `
    precision mediump float;
    varying vec2 v_texCoord;
    uniform sampler2D u_image;
    
    void main() {
      gl_texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, u_image);
      gl_fragColor = texture2D(u_image, v_texCoord);
    }
  `;
  
  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    
    // 获取 WebGL 2 上下文（优先）或 WebGL 1
    let gl = canvas.getContext('webgl2');
    if (!gl) {
      gl = canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
    }
    
    if (!gl) {
      throw new Error('WebGL not supported');
    }
    
    this.gl = gl;
    this.initialize();
  }
  
  private initialize(): void {
    const gl = this.gl;
    
    // 创建着色器程序
    this.program = this.createProgram(
      WebGLRenderer.vertexShaderSource,
      WebGLRenderer.fragmentShaderSource
    );
    
    if (!this.program) {
      throw new Error('Failed to create shader program');
    }
    
    // 创建顶点缓冲区
    const positions = new Float32Array([
      -1, -1,  0, 1,
       1, -1,  1, 1,
      -1,  1,  0, 0,
       1,  1,  1, 0,
    ]);
    
    this.positionBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, this.positionBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, positions, gl.STATIC_DRAW);
    
    // 创建纹理
    this.texture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    
    // 启用深度测试
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.LEQUAL);
  }
  
  private createShader(type: number, source: string): WebGLShader | null {
    const gl = this.gl;
    const shader = gl.createShader(type);
    
    if (!shader) return null;
    
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      console.error('Shader compile error:', gl.getShaderInfoLog(shader));
      gl.deleteShader(shader);
      return null;
    }
    
    return shader;
  }
  
  private createProgram(vertexSource: string, fragmentSource: string): WebGLProgram | null {
    const gl = this.gl;
    
    const vertexShader = this.createShader(gl.VERTEX_SHADER, vertexSource);
    const fragmentShader = this.createShader(gl.FRAGMENT_SHADER, fragmentSource);
    
    if (!vertexShader || !fragmentShader) return null;
    
    const program = gl.createProgram();
    if (!program) return null;
    
    gl.attachShader(program, vertexShader);
    gl.attachShader(program, fragmentShader);
    gl.linkProgram(program);
    
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error('Program link error:', gl.getProgramInfoLog(program));
      gl.deleteProgram(program);
      return null;
    }
    
    return program;
  }
  
  /**
   * 渲染 JPEG/PNG 图像
   */
  public renderImage(imageData: string | ImageData | HTMLImageElement): void {
    const gl = this.gl;
    
    gl.useProgram(this.program);
    
    // 绑定纹理并上传图像数据
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    
    if (typeof imageData === 'string') {
      // Base64 字符串
      const img = new Image();
      img.src = imageData;
      
      if (img.complete) {
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
      }
    } else if (imageData instanceof HTMLImageElement) {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, imageData);
    } else {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, imageData);
    }
    
    // 设置顶点属性
    const positionLoc = gl.getAttribLocation(this.program, 'a_position');
    const texCoordLoc = gl.getAttribLocation(this.program, 'a_texCoord');
    
    gl.bindBuffer(gl.ARRAY_BUFFER, this.positionBuffer);
    
    gl.enableVertexAttribArray(positionLoc);
    gl.vertexAttribPointer(positionLoc, 2, gl.FLOAT, false, 16, 0);
    
    gl.enableVertexAttribArray(texCoordLoc);
    gl.vertexAttribPointer(texCoordLoc, 2, gl.FLOAT, false, 16, 8);
    
    // 清空画布并绘制
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.clearColor(0, 0, 0, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }
  
  /**
   * 渲染检测框
   */
  public renderBoxes(boxes: RenderBox[]): void {
    const gl = this.gl;
    const ctx2d = this.canvas.getContext('2d') as CanvasRenderingContext2D;
    
    // 使用 Canvas 2D 渲染检测框（更简单高效）
    if (!ctx2d) {
      console.error('Failed to get 2D context for box rendering');
      return;
    }
    
    const colors = [
      '#FF6B6B', '#4ECDC4', '#45B7D1', '#96CEB4',
      '#FFEAA7', '#DDA0DD', '#98D8C8', '#F7DC6F'
    ];
    
    boxes.forEach((box, index) => {
      const color = colors[box.class_id % colors.length];
      
      // 绘制矩形
      ctx2d.strokeStyle = color;
      ctx2d.lineWidth = 3;
      ctx2d.strokeRect(box.x, box.y, box.width, box.height);
      
      // 绘制标签
      const label = `${box.class_name} ${(box.confidence || 0.9).toFixed(2)}`;
      ctx2d.font = 'bold 16px Arial';
      const textMetrics = ctx2d.measureText(label);
      const textHeight = 22;
      
      ctx2d.fillStyle = color;
      ctx2d.fillRect(box.x, box.y - textHeight - 4, textMetrics.width + 12, textHeight + 4);
      
      ctx2d.fillStyle = '#000000';
      ctx2d.fillText(label, box.x + 6, box.y - 8);
      
      // 绘制类别 ID 圆圈
      ctx2d.fillStyle = color;
      ctx2d.beginPath();
      ctx2d.arc(box.x + 15, box.y + 15, 16, 0, Math.PI * 2);
      ctx2d.fill();
      
      ctx2d.fillStyle = '#FFFFFF';
      ctx2d.font = 'bold 14px Arial';
      ctx2d.fillText(String(box.class_id), box.x + 9, box.y + 20);
    });
  }
  
  /**
   * 清空画布
   */
  public clear(): void {
    const gl = this.gl;
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  }
  
  /**
   * 调整画布大小
   */
  public resize(width: number, height: number): void {
    this.canvas.width = width;
    this.canvas.height = height;
    this.gl.viewport(0, 0, width, height);
  }
  
  /**
   * 销毁资源
   */
  public destroy(): void {
    const gl = this.gl;
    
    if (this.program) {
      gl.deleteProgram(this.program);
    }
    if (this.texture) {
      gl.deleteTexture(this.texture);
    }
    if (this.positionBuffer) {
      gl.deleteBuffer(this.positionBuffer);
    }
  }
}

/**
 * 渲染框数据结构
 */
export interface RenderBox {
  x: number;
  y: number;
  width: number;
  height: number;
  class_id: number;
  class_name: string;
  confidence: number;
}

/**
 * 优化的渲染管理器
 */
export class RenderManager {
  private renderer: WebGLRenderer | null = null;
  private canvas: HTMLCanvasElement;
  private lastFrame: string | null = null;
  private frameCache: Map<string, ImageData> = new Map();
  
  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
  }
  
  /**
   * 初始化渲染器
   */
  public initialize(): void {
    try {
      this.renderer = new WebGLRenderer(this.canvas);
      console.log('[RenderManager] WebGL renderer initialized');
    } catch (error) {
      console.warn('[RenderManager] WebGL not available, falling back to Canvas 2D');
      this.renderer = null;
    }
  }
  
  /**
   * 渲染帧
   */
  public renderFrame(imageData: string): void {
    // 检查是否需要重新渲染
    if (imageData === this.lastFrame) {
      return;
    }
    
    if (this.renderer) {
      this.renderer.renderImage(imageData);
    } else {
      // 降级到 Canvas 2D
      this.renderFrameCanvas2D(imageData);
    }
    
    this.lastFrame = imageData;
  }
  
  /**
   * Canvas 2D 回退渲染
   */
  private renderFrameCanvas2D(imageData: string): void {
    const ctx = this.canvas.getContext('2d');
    if (!ctx) return;
    
    const img = new Image();
    img.onload = () => {
      ctx.drawImage(img, 0, 0, this.canvas.width, this.canvas.height);
    };
    img.src = imageData;
  }
  
  /**
   * 渲染检测框
   */
  public renderBoxes(boxes: RenderBox[]): void {
    if (this.renderer) {
      this.renderer.renderBoxes(boxes);
    } else {
      this.renderBoxesCanvas2D(boxes);
    }
  }
  
  /**
   * Canvas 2D 渲染检测框
   */
  private renderBoxesCanvas2D(boxes: RenderBox[]): void {
    const ctx = this.canvas.getContext('2d');
    if (!ctx) return;
    
    const colors = [
      '#FF6B6B', '#4ECDC4', '#45B7D1', '#96CEB4',
      '#FFEAA7', '#DDA0DD', '#98D8C8', '#F7DC6F'
    ];
    
    boxes.forEach((box) => {
      const color = colors[box.class_id % colors.length];
      
      ctx.strokeStyle = color;
      ctx.lineWidth = 3;
      ctx.strokeRect(box.x, box.y, box.width, box.height);
      
      const label = `${box.class_name} ${(box.confidence || 0.9).toFixed(2)}`;
      ctx.font = 'bold 16px Arial';
      const textMetrics = ctx.measureText(label);
      const textHeight = 22;
      
      ctx.fillStyle = color;
      ctx.fillRect(box.x, box.y - textHeight - 4, textMetrics.width + 12, textHeight + 4);
      
      ctx.fillStyle = '#000000';
      ctx.fillText(label, box.x + 6, box.y - 8);
    });
  }
  
  /**
   * 调整大小
   */
  public resize(width: number, height: number): void {
    this.canvas.width = width;
    this.canvas.height = height;
    
    if (this.renderer) {
      this.renderer.resize(width, height);
    }
  }
  
  /**
   * 清空
   */
  public clear(): void {
    const ctx = this.canvas.getContext('2d');
    if (ctx) {
      ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    }
    
    if (this.renderer) {
      this.renderer.clear();
    }
    
    this.lastFrame = null;
  }
  
  /**
   * 销毁
   */
  public destroy(): void {
    if (this.renderer) {
      this.renderer.destroy();
      this.renderer = null;
    }
    
    this.frameCache.clear();
  }
}
