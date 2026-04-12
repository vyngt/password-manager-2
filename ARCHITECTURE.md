# VLock Password Manager - Architecture Documentation

## Overview

VLock is a modern, secure password manager built with Rust and Tauri, featuring a clean architecture that separates concerns across multiple components. The application follows a layered architecture pattern with clear boundaries between the user interface, business logic, and data persistence layers.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        VLock Application                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │    App      │    │  Frontend   │    │     UI      │         │
│  │ (Tauri)     │◄──►│ (Leptos)    │◄──►│ (Components)│         │
│  │             │    │             │    │             │         │
│  └─────────────┘    └─────────────┘    └─────────────┘         │
│         │                   │                   │               │
│         ▼                   ▼                   ▼               │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    Backend (Business Logic)                 │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │ │
│  │  │   Domain    │  │  Use Cases  │  │Infrastructure│         │ │
│  │  │             │  │             │  │             │         │ │
│  │  │ • Entities  │  │ • Vault     │  │ • Data      │         │ │
│  │  │ • Services  │  │ • Utilities │  │ • Services  │         │ │
│  │  │ • Repos     │  │             │  │             │         │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘         │ │
│  └─────────────────────────────────────────────────────────────┘ │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────┐    ┌─────────────┐                            │
│  │   Codegen   │    │   SQLite    │                            │
│  │ (Macros)    │    │ (Database)  │                            │
│  └─────────────┘    └─────────────┘                            │
└─────────────────────────────────────────────────────────────────┘
```

## Component Architecture

### 1. App (`/app`) - Tauri Application Layer

**Purpose**: Desktop application wrapper and system integration

**Key Responsibilities**:

- Tauri application setup and configuration
- System-level integrations (notifications, file system, clipboard, etc.)
- Command handling and API exposure
- Application state management
- Plugin management

**Key Files**:

- `src/lib.rs` - Main application entry point and setup
- `src/commands/` - Tauri commands for frontend-backend communication
- `src/config/` - Application configuration and settings
- `src/store/` - Application state management
- `tauri.conf.json` - Tauri configuration

**Dependencies**:

- Tauri 2.x with various plugins (shell, notification, dialog, etc.)
- Backend crate for business logic
- Tokio for async runtime

### 2. Frontend (`/frontend`) - User Interface Layer

**Purpose**: Web-based user interface built with Leptos

**Key Responsibilities**:

- User interface rendering and interaction
- State management for UI components
- API communication with backend
- Routing and navigation
- Feature-specific logic (authentication, window management)

**Key Files**:

- `src/main.rs` - Application entry point
- `src/app.rs` - Main application component
- `src/pages/` - Page components
- `src/features/` - Feature-specific modules
- `src/stores/` - State management
- `src/api/` - Backend communication

**Dependencies**:

- Leptos 0.8.3 (CSR mode)
- Leptos Router for navigation
- Reactive Stores for state management
- UI crate for shared components
- Web-sys for DOM manipulation

**Build Configuration**:

- Trunk for building and serving
- Tailwind CSS 4.1.11 for styling
- Development server on port 1420

### 3. UI (`/ui`) - Shared Component Library

**Purpose**: Reusable UI components and design system

**Key Responsibilities**:

- Shared component library
- Design system implementation
- Styling and theming
- Component variants and states

**Key Components**:

- `components/button/` - Button component with variants
- `components/input/` - Input field components
- `components/icon/` - Icon system
- `components/toast/` - Notification system
- `components/modal/` - Modal dialogs
- `primitives/color/` - Color system

**Dependencies**:

- Leptos for component framework
- Leptos Icons for icon system
- Web-sys for DOM interactions

### 4. Backend (`/backend`) - Business Logic Layer

**Purpose**: Core business logic, domain models, and data access

**Architecture Pattern**: Clean Architecture with clear separation of concerns

#### 4.1 Domain Layer (`business/domain/`)

**Entities**:

- `vault_item.rs` - Password vault item entity
- `folder.rs` - Folder organization entity
- `theme.rs` - Theme and color scheme entity

**Services**:

- `vault.rs` - Vault management service
- `utilities.rs` - Utility services

**Repositories**:

- `vault_item.rs` - Vault item data access interface

#### 4.2 Use Cases Layer (`business/usecases/`)

**Vault Operations**:

- `unlock_vault.rs` - Vault authentication
- `create_vault_item.rs` - Create new vault items
- `update_vault_item.rs` - Update existing items
- `delete_vault_item.rs` - Remove vault items
- `get_vault_item.rs` - Retrieve specific items
- `list_vault_items.rs` - List all vault items
- `change_key_vault.rs` - Change vault master key

**Utilities**:

- `generate_password.rs` - Password generation logic

#### 4.3 Infrastructure Layer (`infra/`)

**Data Layer**:

- SQLite database with SQLCipher encryption
- Sea-ORM for database operations
- Migration system for schema management
- Repository implementations

**Service Layer**:

- `vault.rs` - Vault service implementation
- `password_generator.rs` - Password generation service

**Key Dependencies**:

- Sea-ORM with SQLite support
- SQLCipher for database encryption
- Argon2 for password hashing
- AES-GCM for encryption
- Chrono for date/time handling
- UUID for unique identifiers

### 5. Codegen (`/codegen`) - Code Generation

**Purpose**: Procedural macros for code generation

**Key Features**:

- `StringEnum` derive macro for string-based enums
- Custom code generation utilities

**Dependencies**:

- proc-macro2, quote, syn for macro development

## Data Flow

### 1. User Interaction Flow

```
User Action → Frontend Component → API Call → Tauri Command → Backend Use Case → Domain Service → Repository → Database
```

### 2. Authentication Flow

```
1. User enters master password
2. Frontend sends unlock request via Tauri command
3. Backend derives KEK from master password + salt using KDF (Argon2)
4. Backend uses KEK to decrypt the stored DEK
5. Backend uses DEK to decrypt the database file
6. If successful, vault is unlocked and session established
7. User can access vault items (database operations use DEK)
8. Keys (KEK, DEK) exist only in RAM during session
```

### 3. Vault Item Management

```
1. User creates/updates vault item
2. Data is stored in SQLite database (already encrypted at file level by DEK)
3. Database operations use the in-memory DEK for encryption/decryption
4. UI reflects changes through reactive state management
5. All data remains encrypted at rest via database-level encryption
```

## Security Architecture

### Multi-Layer Encryption Strategy

The application implements a sophisticated two-layer encryption system for maximum security:

#### Layer 1: Database-Level Encryption (DEK)

- **DEK (Data Encryption Key)**: Encrypts the entire database file
- **SQLCipher**: Provides full database encryption using the DEK
- **Database Protection**: The entire SQLite database is encrypted at the file level

#### Layer 2: Key Management (KEK)

- **KEK (Key Encryption Key)**: Encrypts the DEK for secure storage
- **KDF (Key Derivation Function)**: Derives KEK from master password
- **Master Password + Salt + KDF → KEK**: Secure key derivation process
- **Encrypted DEK Storage**: DEK is stored encrypted using KEK

#### Encryption Flow

```
1. Master Password + Salt + KDF → KEK (Key Encryption Key)
2. KEK → Decrypt DEK → DEK (Data Encryption Key) [RAM Only]
3. DEK → Encrypt/Decrypt entire database file
4. KEK and DEK exist only in RAM during session
```

### Data Protection

- **Full Database Encryption**: Entire database file is encrypted with DEK
- **Key Separation**: KEK and DEK are separate keys with different purposes
- **RAM-Only Keys**: Encryption keys exist only in memory during active session
- **Secure Key Derivation**: Argon2-based KDF for master password processing
- **No Plaintext Storage**: No sensitive data stored in plaintext anywhere
- **Session-Based Security**: Keys are cleared from memory when vault is locked

## Development Workflow

### Build Process

1. **Frontend**: Trunk builds Leptos application to WASM
2. **Backend**: Cargo compiles Rust backend
3. **App**: Tauri bundles everything into desktop application
4. **UI**: Shared components are compiled with frontend

### Development Server

- Frontend development server runs on `http://localhost:1420`
- Hot reload enabled for rapid development
- Tauri development mode with live frontend updates

## Technology Stack

### Core Technologies

- **Rust**: Primary language for backend and desktop app
- **Tauri 2.x**: Desktop application framework
- **Leptos 0.8.3**: Frontend framework (CSR mode)
- **SQLite + SQLCipher**: Encrypted database
- **Sea-ORM**: Database ORM and migrations

### Key Libraries

- **Argon2**: Password hashing
- **AES-GCM**: Symmetric encryption
- **Tokio**: Async runtime
- **Serde**: Serialization/deserialization
- **UUID**: Unique identifier generation
- **Chrono**: Date/time handling

### Development Tools

- **Trunk**: Frontend build tool
- **Tailwind CSS**: Styling framework
- **Cargo**: Rust package manager and build system

## Project Structure Benefits

### 1. Separation of Concerns

- Clear boundaries between UI, business logic, and data layers
- Independent development and testing of components
- Easy to maintain and extend

### 2. Reusability

- Shared UI components across the application
- Reusable business logic and use cases
- Modular architecture allows for easy feature additions

### 3. Security

- Encrypted data storage at multiple levels
- Secure key management
- No sensitive data in frontend code

### 4. Performance

- Native desktop application performance
- Efficient Rust backend
- Optimized WASM frontend

### 5. Developer Experience

- Hot reload for rapid development
- Type safety across the entire stack
- Clear project structure and documentation

## Future Considerations

### Scalability

- Architecture supports adding new features easily
- Database migrations handle schema evolution
- Component-based UI allows for feature modules

### Extensibility

- Plugin system through Tauri
- Modular backend allows for new use cases
- Shared UI components support design system evolution

### Maintenance

- Clear separation makes debugging easier
- Comprehensive error handling
- Structured logging and monitoring capabilities
