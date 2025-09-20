# Build UI components:

- Button (/)
- IconButton (/)
- Field
- Form
- Toast (/)
- Modal

# Features:

- Upgrade encryption (even the most powerful computer cannot solve this):

  - Master Password
  - Salt
  - DEK (Data Encryption Key) (Save Encrypted DEK)
  - KEK (Key Encryption Key)
  - KDF (Key Derivation Function)
    => Master Password + KDF + Salt => KEK
    => KEK => Decrypt DEK => KEK (RAM Only) => DB

- Category
- Single Instance
- Auto fill browser?
- I18n

# Work

- Unlock/change master key (/)
- CRUD (Item -> Vault Record) (/)
- Generate Password (/)
- Theme Manager
- Import/Export
- Drop diesel
- App State manager
- Error handling
- Migrate tauri command
- i18n
- Testing
  - Auto test
  - Manually test
