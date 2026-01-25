-- Initial schema setup
-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Channel type enum
CREATE TYPE channel_type AS ENUM ('casual', 'debate');
