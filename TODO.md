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
