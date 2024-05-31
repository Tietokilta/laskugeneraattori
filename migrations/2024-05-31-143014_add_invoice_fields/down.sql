-- This file should undo anything in `up.sql`
ALTER TABLE invoices
DROP COLUMN subject;
ALTER TABLE invoices
DROP COLUMN description;
ALTER TABLE invoices
DROP COLUMN phone_number;
