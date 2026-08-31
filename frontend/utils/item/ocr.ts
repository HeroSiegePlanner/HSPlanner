import { invoke } from '@tauri-apps/api/core'

const BASE64_CHUNK = 0x8000

export async function ocrTooltipImage(blob: Blob): Promise<string[]> {
  const buf = new Uint8Array(await blob.arrayBuffer())
  let bin = ''
  for (let i = 0; i < buf.length; i += BASE64_CHUNK) {
    bin += String.fromCharCode(...buf.subarray(i, i + BASE64_CHUNK))
  }
  return await invoke<string[]>('ocr_tooltip_lines', { imageBase64: btoa(bin) })
}
