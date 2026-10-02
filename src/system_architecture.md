redis-cli
│
│ RESP bytes
▼
┌──────────────┐
│ TCP socket │
└──────┬───────┘
│
▼
┌──────────────┐
│ RESP parser │ ← this new code
└──────┬───────┘
│
▼
["SET", "name", "arnav"]
│
▼
command handling
