# Resumo da Correção: Anexos em Emails via Composio

## Problema Original

O agente ZeroClaw não estava anexando arquivos ao enviar emails via Composio, mesmo após baixar o arquivo do Dropbox com sucesso.

### Exemplo do Problema
```bash
Usuário: "Pegue o arquivo hello.txt do Dropbox e envie para jvvbsj12@gmail.com"

Resultado:
✅ Arquivo encontrado no Dropbox
✅ Arquivo baixado (s3key obtido)
❌ Email enviado SEM anexo (apenas texto informativo)
```

## Causa Raiz (Duas Partes)

### Parte 1: LLM sem instruções sobre anexos
A função `extract_with_llm` não tinha instruções específicas sobre como extrair e incluir anexos.

### Parte 2: Layer 1 muito agressivo
O Layer 1 (quick extraction) estava retornando sucesso mesmo para queries com arquivos/anexos, impedindo que o Layer 2 (LLM) fosse chamado. Como o Layer 1 não tem acesso ao contexto da conversa, ele não pode incluir o campo `attachment`.

## Solução Implementada (Duas Correções)

### Correção 1: Instruções Aprimoradas para o LLM

Adicionadas instruções específicas no prompt do LLM:

```
9. For email attachments: if the query mentions sending/attaching a file AND you see
   file metadata in the conversation context (s3key, mimetype, name from previous tool calls),
   include an 'attachment' field with structure: {"name": "filename", "mimetype": "type", "s3key": "key"}
```

### Correção 2: Bypass do Layer 1 para Queries com Anexos

Adicionada detecção de keywords no Layer 1:

```rust
let attachment_keywords = [
    "file", "arquivo", "attach", "anexo", "anexar",
    "dropbox", "drive", "document", "documento",
    "send file", "enviar arquivo", "with file", "com arquivo"
];

if attachment_keywords.iter().any(|kw| query_lower.contains(kw)) {
    return None; // Force Layer 2 (LLM)
}
```

## Arquivos Modificados

1. **src/tools/composio_nl.rs**
   - Linha ~624: Detecção de keywords e bypass do Layer 1
   - Linha ~680: Instruções aprimoradas no `extract_with_llm`

2. **tests/composio_email_attachment.rs** (novo)
   - 5 testes documentando o comportamento esperado

3. **COMPOSIO_ATTACHMENT_FIX.md** (novo)
   - Documentação técnica da correção inicial

4. **CORRECAO_LAYER1_BYPASS.md** (novo)
   - Documentação da correção do Layer 1

5. **GUIA_ANEXOS_EMAIL.md** (novo)
   - Guia do usuário com exemplos práticos

6. **RESUMO_CORRECAO.md** (este arquivo)
   - Resumo executivo completo

## Como Testar

### Teste Automatizado
```bash
cargo test --test composio_email_attachment
```

Resultado esperado: `5 passed; 0 failed`

### Teste Manual
```bash
# Recompilar
cargo build --release

# Com logs detalhados
$env:RUST_LOG="zeroclaw=debug"

# Comando de teste
zeroclaw agent -m "Pegue o arquivo hello.txt do Dropbox e envie para jvvbsj12@gmail.com"
```

### Verificação nos Logs

Procure por estas linhas (em ordem):

1. **Bypass do Layer 1:**
```
DEBUG zeroclaw::tools::composio_nl: Query mentions file/attachment keywords - skipping Layer 1, will use Layer 2 (LLM)
```

2. **Layer 2 com anexo:**
```
INFO zeroclaw::tools::composio_nl: Layer 2: LLM extraction successful
  arguments={"recipient_email":"...","attachment":{"name":"hello.txt","mimetype":"text/plain","s3key":"..."}}
```

Se você ver ambas as linhas, a correção está funcionando! 📎

## Fluxo Corrigido

### Antes (Incorreto):
```
Query: "Envie arquivo do Dropbox"
  ↓
Layer 1: Quick extraction (retorna sucesso)
  ↓
Resultado: {"recipient_email": "...", "body": "..."}
  ↓
Email SEM anexo ❌
```

### Depois (Correto):
```
Query: "Envie arquivo do Dropbox"
  ↓
Layer 1: Detecta "arquivo" → retorna None
  ↓
Layer 2: LLM com contexto completo
  ↓ (vê s3key do DROPBOX_READ_FILE)
Resultado: {
  "recipient_email": "...",
  "attachment": {
    "name": "hello.txt",
    "mimetype": "text/plain",
    "s3key": "268883/..."
  }
}
  ↓
Email COM anexo ✅
```

## Estrutura Correta do Attachment

Segundo a Composio, o campo `attachment` deve ser um `FileUploadable`:

```json
{
  "name": "hello.txt",
  "mimetype": "text/plain",
  "s3key": "268883/dynamic-module-load/READ_FILE/response/e79cb7e2b3389896687491a8811e2abf"
}
```

### Campos Obrigatórios
- `name`: Nome do arquivo
- `mimetype`: Tipo MIME (ex: `text/plain`, `application/pdf`)
- `s3key`: Referência do arquivo "stagiado" (obtido de `DROPBOX_READ_FILE`)

## Fluxo Recomendado

1. **Buscar**: `DROPBOX_SEARCH_FILE_OR_FOLDER` → encontra o arquivo
2. **Baixar**: `DROPBOX_READ_FILE` → retorna `s3key`, `mimetype`, `name`
3. **Enviar**: `GMAIL_SEND_EMAIL` → usa os dados do passo 2 no campo `attachment`

## Limitações Conhecidas

1. **Contexto do LLM**: O modelo precisa "ver" o resultado do `DROPBOX_READ_FILE` para extrair o `s3key`
2. **Tamanho**: Gmail limita anexos a ~25MB
3. **Keywords**: Atualmente suporta português e inglês
4. **False Positives**: Queries que mencionam "file" mas não são sobre anexos usarão Layer 2 desnecessariamente (mais lento, mas funciona)

## Status da Correção

- ✅ Código modificado (2 correções)
- ✅ Testes criados e passando
- ✅ Documentação técnica criada
- ✅ Guia do usuário criado
- ✅ Build de release bem-sucedido
- ✅ Commits enviados para GitHub

## Commits

1. **`53b662df`** - feat(composio): add email attachment support for Dropbox files
   - Instruções do LLM para anexos
   - Documentação inicial
   - Testes automatizados

2. **`004eba27`** - fix(composio): force Layer 2 (LLM) for email queries with file/attachment keywords
   - Detecção de keywords no Layer 1
   - Bypass para forçar uso do Layer 2
   - Correção do problema de Layer 1 muito agressivo

## Próximos Passos

1. **Testar em produção** com casos reais
2. **Monitorar logs** para verificar se Layer 2 está sendo usado
3. **Coletar feedback** dos usuários
4. **Considerar melhorias**:
   - Suporte a múltiplos anexos
   - Validação de tamanho antes do envio
   - Fallback para link compartilhado se arquivo for muito grande
   - Keywords para outros idiomas (espanhol, francês, etc.)
   - ML para detecção mais inteligente de intenção de anexo

## Referências

- Composio API: https://docs.composio.dev/
- Gmail API: https://developers.google.com/gmail/api
- Dropbox API: https://www.dropbox.com/developers/documentation

## Contato

Para dúvidas ou problemas:
1. Verifique `GUIA_ANEXOS_EMAIL.md` para troubleshooting
2. Execute com `RUST_LOG=zeroclaw=debug` para logs detalhados
3. Abra uma issue no GitHub com logs completos
