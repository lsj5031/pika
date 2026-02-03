export interface DiffData {
  filePath?: string
  language: string
  oldContent: string
  newContent: string
  diffUrl?: string
}

export function parseDiffFromMessage(content: string): DiffData | null {
  if (!content || !content.trim()) return null

  const jsonResult = parseToolCallJson(content)
  if (jsonResult) return jsonResult

  const markdownResult = parseMarkdownCodeBlocks(content)
  if (markdownResult) return markdownResult

  return null
}

function parseToolCallJson(content: string): DiffData | null {
  let cleaned = content.trim()
  const prefixes = [
    /^Tool Result \(Error\):/i,
    /^Tool Result:/i,
    /^Tool Call:/i,
    /^Tool call:/i
  ]

  for (const prefix of prefixes) {
    if (prefix.test(cleaned)) {
      cleaned = cleaned.replace(prefix, '').trim()
    }
  }

  if (cleaned.startsWith('{') || cleaned.startsWith('[')) {
    const result = parseJsonOrFallback(cleaned)
    if (result) return result
  }

  const start = cleaned.indexOf('(')
  const end = cleaned.lastIndexOf(')')
  
  if (start !== -1 && end !== -1 && end > start) {
      const inner = cleaned.substring(start + 1, end).trim()
      if (inner.startsWith('{') || inner.startsWith('[')) {
          const result = parseJsonOrFallback(inner)
          if (result) return result
      }
  }

  return null
}

function parseJsonOrFallback(jsonString: string): DiffData | null {
    try {
        const data = JSON.parse(jsonString)
        const result = extractFromParsedData(data)
        if (result) return result
    } catch {
        console.debug('JSON parse failed, trying regex');
    }

    return regexExtractDiffData(jsonString)
}

function extractFromParsedData(data: Record<string, unknown> | unknown[]): DiffData | null {
    if (Array.isArray(data)) {
      if (data.length === 0) return null
      data = data[0] as Record<string, unknown>
    }

    const root = data as Record<string, unknown>;
    if (root.function && typeof root.function === 'object') {
        const func = root.function as Record<string, unknown>;
        if (func.arguments) {
            if (typeof func.arguments === 'string') {
                try {
                    const parsedArgs = JSON.parse(func.arguments) as Record<string, unknown> | unknown[];
                    return extractFromParsedData(parsedArgs)
                } catch {
                    return regexExtractDiffData(func.arguments as string)
                }
            } else if (typeof func.arguments === 'object' && func.arguments !== null) {
                return extractFromParsedData(func.arguments as Record<string, unknown> | unknown[])
            }
        }
    } else if (root.arguments) {
         if (typeof root.arguments === 'string') {
             try {
                const parsedArgs = JSON.parse(root.arguments) as Record<string, unknown> | unknown[];
                return extractFromParsedData(parsedArgs)
             } catch {
                return regexExtractDiffData(root.arguments as string)
             }
         } else if (typeof root.arguments === 'object' && root.arguments !== null) {
             return extractFromParsedData(root.arguments as Record<string, unknown> | unknown[])
         }
    }

    if (typeof root.TargetFile === 'string' && (root.ReplacementContent !== undefined || root.TargetContent !== undefined)) {
      if (root.ReplacementContent !== undefined && root.TargetContent !== undefined) {
         return {
            filePath: root.TargetFile,
            language: getLanguageFromFilePath(root.TargetFile),
            oldContent: String(root.TargetContent),
            newContent: String(root.ReplacementContent)
         }
      }
    }

    if (typeof root.TargetFile === 'string' && root.CodeContent !== undefined) {
      return {
        filePath: root.TargetFile,
        language: getLanguageFromFilePath(root.TargetFile),
        oldContent: '',
        newContent: String(root.CodeContent)
      }
    }
    
    return null
}

function regexExtractDiffData(text: string): DiffData | null {
    const targetFileMatch = text.match(/"TargetFile"\s*:\s*"([^"]+)"/)
    if (!targetFileMatch) return null
    const filePath = targetFileMatch[1]

    const codeContentMatch = text.match(/"CodeContent"\s*:\s*"((?:[^"\\]|\\.)*)"/)
    if (codeContentMatch) {
        const rawContent = codeContentMatch[1]
        return {
            filePath,
            language: getLanguageFromFilePath(filePath),
            oldContent: '',
            newContent: unescapeJsonString(rawContent)
        }
    }

    const replacementMatch = text.match(/"ReplacementContent"\s*:\s*"((?:[^"\\]|\\.)*)"/)
    const targetMatch = text.match(/"TargetContent"\s*:\s*"((?:[^"\\]|\\.)*)"/)

    if (replacementMatch && targetMatch) {
        return {
            filePath,
            language: getLanguageFromFilePath(filePath),
            oldContent: unescapeJsonString(targetMatch[1]),
            newContent: unescapeJsonString(replacementMatch[1])
        }
    }

    return null
}

function unescapeJsonString(str: string): string {
    return str
        .replace(/\\"/g, '"')
        .replace(/\\\\/g, '\\')
        .replace(/\\n/g, '\n')
        .replace(/\\t/g, '\t')
        .replace(/\\r/g, '\r')
        .replace(/\\b/g, '\b')
        .replace(/\\f/g, '\f')
        .replace(/\\\//g, '/')
}

function parseMarkdownCodeBlocks(content: string): DiffData | null {
  let cleaned = content.trim()
  const prefixes = [
    /^Tool Result \(Error\):/i,
    /^Tool Result:/i,
    /^Tool Call:/i,
    /^Tool call:/i
  ]

  for (const prefix of prefixes) {
    if (prefix.test(cleaned)) {
      cleaned = cleaned.replace(prefix, '').trim()
    }
  }

  const codeBlockRegex = /```(\w+)?\n([\s\S]*?)\n```/g
  
  const matches = [...cleaned.matchAll(codeBlockRegex)]
  
  if (matches.length < 2) {
    return null
  }

  const match1 = matches[0]
  const match2 = matches[1]

  const lang1 = match1[1] || 'text'
  const content1 = match1[2]
  const content2 = match2[2]

  const content1Trimmed = content1.trim()
  const content2Trimmed = content2.trim()

  let filePath: string | undefined
  const firstLine = match1[2].trim().split('\n')[0].trim()
  
  if (firstLine.startsWith('// ')) {
      const potentialPath = firstLine.substring(3).trim()
      if (potentialPath.includes('/') || potentialPath.includes('.')) {
          filePath = potentialPath
      }
  }
  
  return {
    filePath,
    language: lang1 || 'text',
    oldContent: content1Trimmed,
    newContent: content2Trimmed
  }
}

function getLanguageFromFilePath(filePath: string): string {
    const parts = filePath.split('.')
    if (parts.length < 2) return 'text'
    
    const ext = parts.pop()?.toLowerCase()
    if (!ext) return 'text'
    
    const map: Record<string, string> = {
        'ts': 'ts',
        'tsx': 'tsx',
        'js': 'js',
        'jsx': 'jsx',
        'py': 'py',
        'rs': 'rs',
        'go': 'go',
        'java': 'java',
        'cpp': 'cpp',
        'c': 'c',
        'rb': 'rb',
        'php': 'php',
        'swift': 'swift',
        'kt': 'kt',
        'scala': 'scala',
    }
    return map[ext] || ext
}
