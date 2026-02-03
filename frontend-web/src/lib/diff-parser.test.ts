import { describe, it, expect } from 'vitest'
import { parseDiffFromMessage } from './diff-parser'

/**
 * TDD RED PHASE: Comprehensive test suite for diff parser
 *
 * These tests are DESIGNED TO FAIL because the diff-parser module doesn't exist yet.
 * After running these tests and confirming they fail, implement parseDiffFromMessage
 * in diff-parser.ts to make them pass (TDD GREEN phase).
 */

describe('parseDiffFromMessage - Markdown Code Blocks Pattern', () => {
  it('should parse two consecutive markdown code blocks with file path comment', () => {
    const content = `Tool Call: replace_file_content(
      {"TargetFile":"/path/to/file.ts","ReplacementContent":"export function hello() { console.log('new'); }","TargetContent":"export function hello() { console.log('old'); }"}
    )`

    // This will fail initially - parser doesn't exist
    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.filePath).toBe('/path/to/file.ts')
    expect(result?.language).toBe('ts')
    expect(result?.oldContent).toBe('export function hello() { console.log(\'old\'); }')
    expect(result?.newContent).toBe('export function hello() { console.log(\'new\'); }')
  })

  it('should parse markdown code blocks without file path comment', () => {
    const content = `Some text before
\`\`\`typescript
const old = "value"
\`\`\`

\`\`\`typescript
const new = "updated"
\`\`\`
Some text after`

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.filePath).toBeUndefined()
    expect(result?.language).toBe('typescript')
    expect(result?.oldContent).toBe('const old = "value"')
    expect(result?.newContent).toBe('const new = "updated"')
  })

  it('should parse code blocks with default language (text)', () => {
    const content = `Old code:
\`\`\`
line 1
line 2
\`\`\`

New code:
\`\`\`
line 1
line 2 changed
\`\`\``

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.language).toBe('text')
    expect(result?.oldContent).toBe('line 1\nline 2')
    expect(result?.newContent).toBe('line 1\nline 2 changed')
  })

  it('should handle code blocks with various languages', () => {
    const languages = [
      'javascript', 'typescript', 'python', 'rust', 'go', 'java',
      'cpp', 'c', 'ruby', 'php', 'swift', 'kotlin', 'scala'
    ]

    languages.forEach(lang => {
      const content = `\`\`\`${lang}
old code in ${lang}
\`\`\`

\`\`\`${lang}
new code in ${lang}
\`\`\``

      const result = parseDiffFromMessage(content)

      expect(result).not.toBeNull()
      expect(result?.language).toBe(lang)
    })
  })

  it('should trim whitespace from code block content', () => {
    const content = `
\`\`\`typescript
    const old = {
      indented: "code"
    }
    
\`\`\`

\`\`\`typescript
    const new = {
      indented: "code"
    }
    
\`\`\`
    `

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.oldContent).not.toMatch(/^\s+/)
    expect(result?.newContent).not.toMatch(/^\s+/)
  })
})

describe('parseDiffFromMessage - Tool Call JSON Pattern (ReplacementContent)', () => {
  it('should parse JSON with ReplacementContent and TargetContent', () => {
    const content = `Tool Call: replace_file_content({
      "TargetFile": "src/components/Button.tsx",
      "ReplacementContent": "export const Button = () => <button>New</button>",
      "TargetContent": "export const Button = () => <button>Old</button>"
    })`

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.filePath).toBe('src/components/Button.tsx')
    expect(result?.oldContent).toBe('export const Button = () => <button>Old</button>')
    expect(result?.newContent).toBe('export const Button = () => <button>New</button>')
  })

  it('should parse JSON without function wrapper (direct object)', () => {
    const content = `Tool call: {
      "TargetFile": "lib/utils.ts",
      "ReplacementContent": "export function new() {}",
      "TargetContent": "export function old() {}"
    }`

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.filePath).toBe('lib/utils.ts')
    expect(result?.oldContent).toBe('export function old() {}')
    expect(result?.newContent).toBe('export function new() {}')
  })

  it('should parse JSON with function.arguments format', () => {
    const content = `Tool Call: some_tool({
      "function": {
        "name": "replace_file_content",
        "arguments": '{"TargetFile":"api.ts","ReplacementContent":"new","TargetContent":"old"}'
      }
    })`

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.filePath).toBe('api.ts')
    expect(result?.oldContent).toBe('old')
    expect(result?.newContent).toBe('new')
  })

  it('should parse JSON with arguments field', () => {
    const content = `Tool Call: edit_file({
      "arguments": {
        "TargetFile": "config.json",
        "ReplacementContent": "{ \\"key\\": \\"new\\" }",
        "TargetContent": "{ \\"key\\": \\"old\\" }"
      }
    })`

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.filePath).toBe('config.json')
    expect(result?.oldContent).toBe('{ "key": "old" }')
    expect(result?.newContent).toBe('{ "key": "new" }')
  })
})

describe('parseDiffFromMessage - Tool Call JSON Pattern (CodeContent)', () => {
  it('should parse JSON with CodeContent (new file creation)', () => {
    const content = `Tool Call: create_new_file({
      "TargetFile": "src/new_file.ts",
      "CodeContent": "export const NEW_FILE = true;\nexport const version = '1.0.0';"
    })`

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.filePath).toBe('src/new_file.ts')
    expect(result?.language).toBe('ts')
    expect(result?.oldContent).toBe('') // Empty for new files
    expect(result?.newContent).toBe('export const NEW_FILE = true;\nexport const version = \'1.0.0\';')
  })

  it('should handle CodeContent with various file extensions', () => {
    const cases = [
      { file: 'script.py', lang: 'py' },
      { file: 'main.rs', lang: 'rs' },
      { file: 'app.go', lang: 'go' },
      { file: 'Component.jsx', lang: 'jsx' },
      { file: 'styles.css', lang: 'css' },
    ]

    cases.forEach(({ file, lang }) => {
      const content = `Tool Call: create_file({
        "TargetFile": "${file}",
        "CodeContent": "content here"
      })`

      const result = parseDiffFromMessage(content)

      expect(result).not.toBeNull()
      expect(result?.filePath).toBe(file)
      expect(result?.language).toBe(lang)
      expect(result?.oldContent).toBe('')
    })
  })
})

describe('parseDiffFromMessage - Tool Result Prefix Handling', () => {
  it('should strip "Tool Result:" prefix', () => {
    const content = `Tool Result:
\`\`\`typescript
old
\`\`\`

\`\`\`typescript
new
\`\`\``

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.oldContent).toBe('old')
    expect(result?.newContent).toBe('new')
  })

  it('should strip "Tool Result (Error):" prefix', () => {
    const content = `Tool Result (Error):
\`\`\`typescript
old code
\`\`\`

\`\`\`typescript
new code
\`\`\``

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
  })

  it('should handle lowercase "tool call:" prefix', () => {
    const content = `tool call: {
      "TargetFile": "test.ts",
      "ReplacementContent": "new",
      "TargetContent": "old"
    }`

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
  })
})

describe('parseDiffFromMessage - Edge Cases', () => {
  it('should return null for single code block (not enough for diff)', () => {
    const content = `Here's a single code block:
\`\`\`typescript
const only = "one block"
\`\`\``

    const result = parseDiffFromMessage(content)

    expect(result).toBeNull()
  })

  it('should return null for content with no diff patterns', () => {
    const content = 'This is just plain text with no code blocks or JSON.'

    const result = parseDiffFromMessage(content)

    expect(result).toBeNull()
  })

  it('should return null for malformed JSON', () => {
    const content = 'Tool Call: { "broken": json, missing }'

    const result = parseDiffFromMessage(content)

    expect(result).toBeNull()
  })

  it('should return null for JSON without expected fields', () => {
    const content = `Tool Call: some_tool({
      "OtherField": "value",
      "AnotherField": 123
    })`

    const result = parseDiffFromMessage(content)

    expect(result).toBeNull()
  })

  it('should handle empty content gracefully', () => {
    const result = parseDiffFromMessage('')

    expect(result).toBeNull()
  })

  it('should handle whitespace-only content', () => {
    const result = parseDiffFromMessage('   \n\n  \t  ')

    expect(result).toBeNull()
  })

  it('should handle content with only tool result prefix but no diff', () => {
    const content = 'Tool Result: Some error occurred during processing'

    const result = parseDiffFromMessage(content)

    expect(result).toBeNull()
  })
})

describe('parseDiffFromMessage - Complex Real-World Scenarios', () => {
  it('should handle multiline content in code blocks', () => {
    const content = `Changes:
\`\`\`typescript
function oldFunction() {
  console.log('line 1');
  console.log('line 2');
  console.log('line 3');
  return true;
}
\`\`\`

\`\`\`typescript
function newFunction() {
  console.log('line 1');
  console.log('line 2 modified');
  console.log('line 3');
  return false;
}
\`\`\``

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.oldContent).toContain('console.log(\'line 2\');')
    expect(result?.newContent).toContain('console.log(\'line 2 modified\');')
  })

  it('should handle code blocks with special characters', () => {
    const content = `\`\`\`javascript
const str = "hello \\\n world";
const regex = /\\\\d+/g;
const template = \`value: \${x}\`;
\`\`\`

\`\`\`javascript
const str = "hello \\\n world";
const regex = /[a-z]+/g;
const template = \`value: \${y}\`;
\`\`\``

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.oldContent).toContain('/\\\\d+/g')
    expect(result?.newContent).toContain('/[a-z]+/g')
  })

  it('should handle JSON with escaped strings', () => {
    const content = `Tool Call: replace({
      "TargetFile": "test.ts",
      "ReplacementContent": "const s = \\"value\\";\\n// escaped \\\\n",
      "TargetContent": "const s = \\"old\\";\\n// escaped \\\\t"
    })`

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.oldContent).toBe('const s = "old";\n// escaped \\t')
    expect(result?.newContent).toBe('const s = "value";\n// escaped \\n')
  })

  it('should handle tool call with array format (take first item)', () => {
    const content = `Tool Call: tools([
      {
        "function": {
          "arguments": '{"TargetFile":"f.ts","ReplacementContent":"new","TargetContent":"old"}'
        }
      },
      {
        "function": {
          "arguments": '{"other":"field"}'
        }
      }
    ])`

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.filePath).toBe('f.ts')
  })
})

describe('parseDiffFromMessage - File Path Extraction', () => {
  it('should extract language from file extension', () => {
    const extensions: Record<string, string> = {
      'file.ts': 'ts',
      'file.tsx': 'tsx',
      'file.js': 'js',
      'file.jsx': 'jsx',
      'file.py': 'py',
      'file.rs': 'rs',
      'file.go': 'go',
      'file.java': 'java',
      'file.cpp': 'cpp',
      'file.c': 'c',
      'file.rb': 'rb',
      'file.php': 'php',
      'file.swift': 'swift',
      'file.kt': 'kt',
      'file.scala': 'scala',
    }

    Object.entries(extensions).forEach(([file, expectedLang]) => {
      const content = `Tool Call: create({
        "TargetFile": "${file}",
        "CodeContent": "content"
      })`

      const result = parseDiffFromMessage(content)

      expect(result?.language).toBe(expectedLang)
    })
  })

  it('should default to "text" for files without recognizable extension', () => {
    const content = `Tool Call: create({
      "TargetFile": "README",
      "CodeContent": "# Documentation"
    })`

    const result = parseDiffFromMessage(content)

    expect(result?.language).toBe('text')
  })

  it('should handle nested file paths', () => {
    const content = `Tool Call: edit({
      "TargetFile": "src/components/ui/button/PrimaryButton.tsx",
      "ReplacementContent": "new content",
      "TargetContent": "old content"
    })`

    const result = parseDiffFromMessage(content)

    expect(result?.filePath).toBe('src/components/ui/button/PrimaryButton.tsx')
    expect(result?.language).toBe('tsx')
  })
})

describe('parseDiffFromMessage - Multiline Comments in Code Blocks', () => {
  it('should parse code blocks with C-style multiline comments', () => {
    const content = `\`\`\`c
/* old comment */
int main() {}
\`\`\`

\`\`\`c
/* new comment */
int main() {}
\`\`\``

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
    expect(result?.oldContent).toContain('/* old comment */')
    expect(result?.newContent).toContain('/* new comment */')
  })

  it('should parse code blocks with template literals', () => {
    const content = `\`\`\`typescript
const t1 = \`old \${value}\`;
\`\`\`

\`\`\`typescript
const t2 = \`new \${value}\`;
\`\`\``

    const result = parseDiffFromMessage(content)

    expect(result).not.toBeNull()
  })
})
