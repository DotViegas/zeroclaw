# Fix: Composio Email Attachments

## Problema Identificado

O agente ZeroClaw não estava anexando arquivos ao enviar emails via Composio, mesmo quando o arquivo havia sido baixado do Dropbox.

### O que acontecia:
1. ✅ Arquivo encontrado no Dropbox (`DROPBOX_SEARCH_FILE_OR_FOLDER`)
2. ✅ Arquivo baixado com sucesso (`DROPBOX_READ_FILE` retornando `s3key`, `mimetype`, `name`)
3. ❌ Email enviado SEM anexo - apenas texto informativo

### Causa Raiz

A função `try_quick_extraction` no `composio_nl.rs` extraía apenas:
- `recipient_email`
- `subject`
- `body`

O campo `attachment` nunca era processado, mesmo quando o `s3key` estava disponível no contexto.

## Solução Implementada

### 1. Documentação no Layer 1 (Quick Extraction)

Adicionado comentário explicando que anexos NÃO são tratados no Layer 1 porque requerem contexto de chamadas anteriores:

```rust
// NOTE: Attachment extraction is NOT handled here in Layer 1
// because it requires context from previous tool calls (s3key from DROPBOX_READ_FILE, etc.)
// Layer 2 (LLM) should handle attachment context when the query mentions
// "attach file", "send file", "with attachment", etc.
// The LLM can reference previous tool results to extract s3key/mimetype/name
```

### 2. Instruções Aprimoradas no Layer 2 (LLM Extraction)

Adicionadas instruções específicas no prompt do LLM:

```
9. For email attachments: if the query mentions sending/attaching a file AND you see
   file metadata in the conversation context (s3key, mimetype, name from previous tool calls),
   include an 'attachment' field with structure: {"name": "filename", "mimetype": "type", "s3key": "key"}
```

Exemplo adicionado ao prompt:
```json
{
  "recipient_email": "user@example.com",
  "subject": "File",
  "body": "See attached",
  "attachment": {
    "name": "file.txt",
    "mimetype": "text/plain",
    "s3key": "268883/..."
  }
}
```

## Estrutura Correta do Attachment (Composio)

Segundo a documentação da Composio, o campo `attachment` deve ser um `FileUploadable`:

```json
{
  "name": "hello.txt",
  "mimetype": "text/plain",
  "s3key": "268883/dynamic-module-load/READ_FILE/response/e79cb7e2b3389896687491a8811e2abf"
}
```

### Campos obrigatórios:
- `name`: Nome do arquivo
- `mimetype`: Tipo MIME (ex: `text/plain`, `application/pdf`)
- `s3key`: Referência do arquivo "stagiado" (obtido de `DROPBOX_READ_FILE` ou `stage_content`)

## Fluxo Recomendado

Para enviar arquivo do Dropbox por email:

1. **Buscar arquivo**: `DROPBOX_SEARCH_FILE_OR_FOLDER`
2. **Baixar arquivo**: `DROPBOX_READ_FILE` (retorna `s3key`, `mimetype`, `name`)
3. **Enviar email com anexo**: `GMAIL_SEND_EMAIL` usando os dados do passo 2

### Exemplo de query natural:

```
"Pegue o arquivo hello.txt do meu Dropbox e envie para user@example.com"
```

O LLM agora deve:
1. Reconhecer que precisa baixar o arquivo primeiro
2. Extrair o `s3key` do resultado do download
3. Incluir o campo `attachment` no `GMAIL_SEND_EMAIL`

## Teste

Para testar a correção:

```bash
cargo build --release
zeroclaw agent -m "Pegue o arquivo hello.txt do Dropbox e envie para jvvbsj12@gmail.com"
```

Verifique nos logs se o `GMAIL_SEND_EMAIL` inclui o campo `attachment`.

## Limitações Conhecidas

- O LLM precisa ter acesso ao contexto da conversa para ver o `s3key` retornado pelo `DROPBOX_READ_FILE`
- Se o modelo LLM não tiver contexto suficiente, pode não incluir o anexo
- Anexos grandes (>25MB) podem falhar devido a limites do Gmail

## Referências

- Composio Tool Schemas: `COMPOSIO_GET_TOOL_SCHEMAS`
- Composio Staging API: `stage_content` em `composio_nl.rs`
- Gmail Send Email: `GMAIL_SEND_EMAIL` com campo `attachment` opcional
