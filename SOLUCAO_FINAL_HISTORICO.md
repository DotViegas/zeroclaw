# Solução Final: Histórico de Execução para Contexto de Anexos

## Problema Identificado pelo Usuário

O usuário fez uma observação crucial: **"não seria interessante ter o histórico temporário enquanto está rodando esse loop de ComposioNaturalLanguageTool?"**

Essa foi a chave para resolver o problema de forma elegante!

## Análise do Problema Real

O problema fundamental era que o LLM no `extract_with_llm` **não tinha acesso ao histórico da conversa**:

```rust
// Make a quick LLM call (no history, just extraction)
let messages = vec![
    crate::providers::ChatMessage::user(prompt),
];
```

Sem o histórico, o LLM não podia ver:
- O resultado do `DROPBOX_READ_FILE` com o `s3key`
- Os metadados do arquivo (mimetype, name)
- Qualquer contexto de chamadas anteriores

## Solução Implementada

### 1. Adicionado Campo de Histórico

```rust
pub struct ComposioNaturalLanguageTool {
    // ... campos existentes ...
    
    // Execution history for context in LLM extraction
    // Stores recent tool executions (tool_slug, query, result) for attachment context
    execution_history: Arc<RwLock<Vec<(String, String, Value)>>>,
}
```

### 2. Armazenamento Automático de Execuções

Após cada execução bem-sucedida de ferramenta:

```rust
// Store execution in history for context in future LLM extractions
// Keep only last 10 executions to avoid memory bloat
{
    let mut history = self.execution_history.write().await;
    history.push((
        tool_slug.to_string(),
        arguments.to_string(),
        parsed_data.clone(),
    ));
    
    // Keep only last 10 executions
    if history.len() > 10 {
        history.remove(0);
    }
}
```

### 3. Extração de Contexto de Anexos

Nova função para extrair metadados de arquivos do histórico:

```rust
async fn get_attachment_context(&self) -> String {
    let history = self.execution_history.read().await;
    
    let mut context_parts = Vec::new();
    
    for (tool_slug, _args, result) in history.iter().rev().take(5) {
        // Look for DROPBOX_READ_FILE or similar download operations
        if tool_slug.contains("READ_FILE") || tool_slug.contains("DOWNLOAD") {
            // Extract file metadata from result
            if let Some(content) = result.get("data")...get("content") {
                let name = content.get("name")...;
                let mimetype = content.get("mimetype")...;
                let s3key = content.get("s3key")...;
                
                context_parts.push(format!(
                    "- File downloaded: name=\"{}\", mimetype=\"{}\", s3key=\"{}\"",
                    name, mimetype, s3key
                ));
            }
        }
    }
    
    format!("\n\nRECENT FILE DOWNLOADS (for attachment context):\n{}\n", 
            context_parts.join("\n"))
}
```

### 4. Inclusão no Prompt do LLM

O contexto é adicionado ao prompt:

```rust
let attachment_context = self.get_attachment_context().await;

let prompt = format!(
    "Extract parameters from the user query for tool execution.\n\n\
     Tool: {}\n\
     Use case: {}\n\
     Parameter schema:\n{}\n\n\
     User query: \"{}\"{}\n\n\  // <-- attachment_context aqui
     CRITICAL INSTRUCTIONS:\n\
     ...\n\
     9. IMPORTANT FOR EMAIL ATTACHMENTS:\n\
        - If the query mentions 'attach', 'file', 'arquivo', 'anexo'\n\
        - AND the schema has an 'attachment' field\n\
        - AND you see file metadata in the RECENT FILE DOWNLOADS section above\n\
        - THEN you MUST include the 'attachment' field\n\
        - Use the EXACT s3key, mimetype, and name from the RECENT FILE DOWNLOADS\n\
     ...",
    tool_slug, use_case, schema_str, query, attachment_context
);
```

## Fluxo Completo

### Antes (Sem Histórico):
```
1. DROPBOX_READ_FILE executa
   ↓ (resultado perdido)
2. GMAIL_SEND_EMAIL chamado
   ↓
3. LLM extrai parâmetros SEM contexto
   ↓
4. Email enviado SEM anexo ❌
```

### Depois (Com Histórico):
```
1. DROPBOX_READ_FILE executa
   ↓
2. Resultado armazenado no histórico
   {
     "content": {
       "name": "hello.txt",
       "mimetype": "text/plain",
       "s3key": "268883/..."
     }
   }
   ↓
3. GMAIL_SEND_EMAIL chamado
   ↓
4. get_attachment_context() extrai do histórico:
   "RECENT FILE DOWNLOADS:
    - File downloaded: name=\"hello.txt\", mimetype=\"text/plain\", s3key=\"268883/...\""
   ↓
5. LLM vê o contexto no prompt
   ↓
6. LLM extrai:
   {
     "recipient_email": "...",
     "subject": "...",
     "attachment": {
       "name": "hello.txt",
       "mimetype": "text/plain",
       "s3key": "268883/..."
     }
   }
   ↓
7. Email enviado COM anexo ✅
```

## Vantagens da Solução

### 1. Elegante e Simples
- Não requer mudanças na arquitetura do agente
- Histórico é local ao `ComposioNaturalLanguageTool`
- Limpeza automática (mantém apenas últimas 10 execuções)

### 2. Eficiente
- Armazena apenas o necessário
- Busca apenas nos últimos 5 resultados
- Memória limitada (máximo 10 execuções)

### 3. Extensível
- Funciona para qualquer tipo de arquivo (Dropbox, Drive, etc.)
- Pode ser expandido para outros casos de uso
- Fácil adicionar novos padrões de extração

### 4. Transparente
- Logs mostram quando o histórico é atualizado
- Contexto é visível no prompt do LLM
- Fácil debugar

## Limitações

### 1. Escopo por Instância
- Histórico é por instância do `ComposioNaturalLanguageTool`
- Se a ferramenta for recriada, histórico é perdido
- Solução: OK para uso normal, pois a ferramenta persiste durante a conversa

### 2. Tamanho Limitado
- Mantém apenas últimas 10 execuções
- Se houver muitas operações entre download e envio, pode perder contexto
- Solução: 10 é suficiente para casos normais de uso

### 3. Formato Específico
- Atualmente detecta apenas formato Dropbox (`content.s3key`)
- Outros serviços podem ter formatos diferentes
- Solução: Fácil adicionar novos padrões na função `get_attachment_context`

## Como Testar

```bash
# Recompilar
cargo build --release

# Testar
$env:RUST_LOG="zeroclaw=debug"
zeroclaw agent -m "Pegue o arquivo hello.txt do Dropbox e envie para jvvbsj12@gmail.com"
```

### Logs Esperados

1. **Após DROPBOX_READ_FILE:**
```
DEBUG: Updated execution history history_size=1
```

2. **Antes de GMAIL_SEND_EMAIL:**
```
DEBUG: Query mentions file/attachment keywords - skipping Layer 1, will use Layer 2 (LLM)
```

3. **Durante extração do LLM:**
O prompt incluirá:
```
RECENT FILE DOWNLOADS (for attachment context):
- File downloaded: name="hello.txt", mimetype="text/plain", s3key="268883/..."
```

4. **Resultado:**
```
INFO: Layer 2: LLM extraction successful
  arguments={"recipient_email":"...","attachment":{"name":"hello.txt","mimetype":"text/plain","s3key":"268883/..."}}
```

## Commits Relacionados

1. `53b662df` - Correção inicial: instruções do LLM
2. `004eba27` - Bypass do Layer 1 para keywords
3. `39b01510` - Documentação completa
4. `5147819c` - **Esta solução: histórico de execução**

## Créditos

Solução sugerida pelo usuário: **"não seria interessante ter o histórico temporário enquanto está rodando esse loop de ComposioNaturalLanguageTool?"**

Essa observação foi fundamental para identificar e resolver o problema de forma elegante! 🎉
