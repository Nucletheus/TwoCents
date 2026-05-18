-- Seed default categories
-- Comprehensive category seed covering expenses, income, and transfers
INSERT INTO public.categories (name, icon, color, group_name, is_default)
VALUES
  -- Food & Dining
  ('Groceries', NULL, '#FF6B6B', 'Food & Dining', true),
  ('Restaurants', NULL, '#FF8E8E', 'Food & Dining', true),
  ('Fast Food', NULL, '#FFB3B3', 'Food & Dining', true),
  ('Coffee & Tea', NULL, '#8B4513', 'Food & Dining', true),
  ('Alcohol & Bars', NULL, '#9B59B6', 'Food & Dining', true),
  ('Food Delivery', NULL, '#E74C3C', 'Food & Dining', true),
  
  -- Shopping
  ('Clothing & Apparel', NULL, '#4ECDC4', 'Shopping', true),
  ('Electronics', NULL, '#3498DB', 'Shopping', true),
  ('Home & Garden', NULL, '#2ECC71', 'Shopping', true),
  ('Books & Media', NULL, '#E67E22', 'Shopping', true),
  ('Sports & Outdoors', NULL, '#1ABC9C', 'Shopping', true),
  ('Toys & Games', NULL, '#F39C12', 'Shopping', true),
  ('Pet Supplies', NULL, '#16A085', 'Shopping', true),
  
  -- Transportation
  ('Gas & Fuel', NULL, '#45B7D1', 'Transportation', true),
  ('Public Transit', NULL, '#5DADE2', 'Transportation', true),
  ('Parking', NULL, '#85C1E2', 'Transportation', true),
  ('Car Maintenance', NULL, '#AED6F1', 'Transportation', true),
  ('Car Insurance', NULL, '#3498DB', 'Transportation', true),
  ('Rideshare & Taxi', NULL, '#5DADE2', 'Transportation', true),
  ('Tolls', NULL, '#85C1E2', 'Transportation', true),
  
  -- Household & Utilities
  ('Electricity', NULL, '#FFA07A', 'Household', true),
  ('Water', NULL, '#48C9B0', 'Household', true),
  ('Gas & Heating', NULL, '#F39C12', 'Household', true),
  ('Internet', NULL, '#3498DB', 'Household', true),
  ('Phone', NULL, '#9B59B6', 'Household', true),
  ('Cable & Streaming', NULL, '#E74C3C', 'Household', true),
  ('Rent', NULL, '#95A5A6', 'Household', true),
  ('Mortgage', NULL, '#7F8C8D', 'Household', true),
  ('Home Insurance', NULL, '#2C3E50', 'Household', true),
  
  -- Healthcare
  ('Medical & Doctor', NULL, '#F7DC6F', 'Healthcare', true),
  ('Dental', NULL, '#F4D03F', 'Healthcare', true),
  ('Pharmacy', NULL, '#F9E79F', 'Healthcare', true),
  ('Health Insurance', NULL, '#F7DC6F', 'Healthcare', true),
  ('Gym & Fitness', NULL, '#E74C3C', 'Healthcare', true),
  
  -- Personal Care
  ('Haircut & Salon', NULL, '#F1948A', 'Personal Care', true),
  ('Personal Hygiene', NULL, '#EC7063', 'Personal Care', true),
  ('Cosmetics', NULL, '#F5B7B1', 'Personal Care', true),
  
  -- Entertainment
  ('Movies & Cinema', NULL, '#98D8C8', 'Entertainment', true),
  ('Concerts & Events', NULL, '#85C1E2', 'Entertainment', true),
  ('Subscriptions', NULL, '#A8E6CF', 'Entertainment', true),
  ('Hobbies', NULL, '#C8E6C9', 'Entertainment', true),
  
  -- Education
  ('Tuition', NULL, '#BB8FCE', 'Education', true),
  ('School Supplies', NULL, '#D7BDE2', 'Education', true),
  ('Courses & Training', NULL, '#C39BD3', 'Education', true),
  
  -- Travel
  ('Flights', NULL, '#85C1E2', 'Travel', true),
  ('Hotels', NULL, '#AED6F1', 'Travel', true),
  ('Vacation', NULL, '#5DADE2', 'Travel', true),
  
  -- Loans & Debt
  ('Car Payment', NULL, '#E67E22', 'Loans & Debt', true),
  ('Car Loan', NULL, '#D35400', 'Loans & Debt', true),
  ('Student Loans', NULL, '#8E44AD', 'Loans & Debt', true),
  ('Credit Card Payment', NULL, '#9B59B6', 'Loans & Debt', true),
  ('Personal Loan', NULL, '#7D3C98', 'Loans & Debt', true),
  ('Mortgage Payment', NULL, '#7F8C8D', 'Loans & Debt', true),
  ('Line of Credit', NULL, '#95A5A6', 'Loans & Debt', true),
  
  -- Family & Childcare
  ('Childcare', NULL, '#F39C12', 'Family & Childcare', true),
  ('Daycare', NULL, '#F7DC6F', 'Family & Childcare', true),
  ('Babysitting', NULL, '#F4D03F', 'Family & Childcare', true),
  ('School Fees', NULL, '#BB8FCE', 'Family & Childcare', true),
  ('After-School Activities', NULL, '#D7BDE2', 'Family & Childcare', true),
  ('Child Support Paid', NULL, '#EC7063', 'Family & Childcare', true),
  ('Alimony Paid', NULL, '#F1948A', 'Family & Childcare', true),
  
  -- Insurance
  ('Life Insurance', NULL, '#3498DB', 'Insurance', true),
  ('Disability Insurance', NULL, '#5DADE2', 'Insurance', true),
  ('Long-Term Care Insurance', NULL, '#85C1E2', 'Insurance', true),
  
  -- Taxes & Fees
  ('Income Tax', NULL, '#E74C3C', 'Taxes & Fees', true),
  ('Property Tax', NULL, '#34495E', 'Taxes & Fees', true),
  ('Sales Tax', NULL, '#2C3E50', 'Taxes & Fees', true),
  ('Vehicle Registration', NULL, '#7F8C8D', 'Taxes & Fees', true),
  ('License Renewal', NULL, '#95A5A6', 'Taxes & Fees', true),
  ('Bank Fees', NULL, '#95A5A6', 'Taxes & Fees', true),
  ('ATM Fees', NULL, '#BDC3C7', 'Taxes & Fees', true),
  
  -- Professional Services
  ('Legal Fees', NULL, '#7F8C8D', 'Professional Services', true),
  ('Accounting Services', NULL, '#95A5A6', 'Professional Services', true),
  ('Professional Services', NULL, '#BDC3C7', 'Professional Services', true),
  
  -- Home Maintenance
  ('Home Maintenance', NULL, '#2ECC71', 'Home Maintenance', true),
  ('Home Repairs', NULL, '#27AE60', 'Home Maintenance', true),
  ('Appliance Repair', NULL, '#52BE80', 'Home Maintenance', true),
  ('Cleaning Services', NULL, '#58D68D', 'Home Maintenance', true),
  ('Laundry', NULL, '#7DCEA0', 'Home Maintenance', true),
  ('Storage', NULL, '#A9DFBF', 'Home Maintenance', true),
  ('Home Improvement', NULL, '#16A085', 'Home Maintenance', true),
  
  -- Business & Work
  ('Business Expenses', NULL, '#3498DB', 'Business & Work', true),
  ('Office Supplies', NULL, '#5DADE2', 'Business & Work', true),
  ('Software Subscriptions', NULL, '#85C1E2', 'Business & Work', true),
  ('Cloud Storage', NULL, '#AED6F1', 'Business & Work', true),
  ('Domain & Hosting', NULL, '#3498DB', 'Business & Work', true),
  ('Professional Development', NULL, '#5DADE2', 'Business & Work', true),
  ('Work Meals', NULL, '#FF6B6B', 'Business & Work', true),
  ('Work Travel', NULL, '#85C1E2', 'Business & Work', true),
  
  -- Memberships
  ('Gym Membership', NULL, '#E74C3C', 'Memberships', true),
  ('Club Memberships', NULL, '#98D8C8', 'Memberships', true),
  ('Magazine Subscriptions', NULL, '#A8E6CF', 'Memberships', true),
  
  -- Charitable
  ('Gifts & Donations', NULL, '#F8BBD0', 'Charitable', true),
  ('Charitable Donations', NULL, '#F1948A', 'Charitable', true),
  
  -- Savings & Investments
  ('Emergency Fund', NULL, '#27AE60', 'Savings & Investments', true),
  ('Savings Contribution', NULL, '#2ECC71', 'Savings & Investments', true),
  ('Investment Contribution', NULL, '#16A085', 'Savings & Investments', true),
  ('Retirement Contribution', NULL, '#1ABC9C', 'Savings & Investments', true),
  
  -- Miscellaneous
  ('Postage & Shipping', NULL, '#95A5A6', 'Miscellaneous', true),
  ('Other Expenses', NULL, '#BDC3C7', 'Miscellaneous', true),
  
  -- Income - Employment
  ('Salary', NULL, '#27AE60', 'Income - Employment', true),
  ('Part-Time Job', NULL, '#2ECC71', 'Income - Employment', true),
  ('Second Job', NULL, '#16A085', 'Income - Employment', true),
  ('Overtime Pay', NULL, '#1ABC9C', 'Income - Employment', true),
  ('Commission', NULL, '#27AE60', 'Income - Employment', true),
  ('Bonus', NULL, '#52BE80', 'Income - Employment', true),
  ('Tips', NULL, '#58D68D', 'Income - Employment', true),
  
  -- Income - Side Income
  ('Freelance', NULL, '#2ECC71', 'Income - Side Income', true),
  ('Side Hustle', NULL, '#16A085', 'Income - Side Income', true),
  ('Consulting', NULL, '#1ABC9C', 'Income - Side Income', true),
  ('Contract Work', NULL, '#27AE60', 'Income - Side Income', true),
  ('Gig Work', NULL, '#52BE80', 'Income - Side Income', true),
  ('Online Income', NULL, '#58D68D', 'Income - Side Income', true),
  
  -- Income - Business & Investment
  ('Business Income', NULL, '#27AE60', 'Income - Business & Investment', true),
  ('Investment Income', NULL, '#16A085', 'Income - Business & Investment', true),
  ('Rental Income', NULL, '#1ABC9C', 'Income - Business & Investment', true),
  ('Interest & Dividends', NULL, '#A9DFBF', 'Income - Business & Investment', true),
  ('Capital Gains', NULL, '#7DCEA0', 'Income - Business & Investment', true),
  ('Passive Income', NULL, '#52BE80', 'Income - Business & Investment', true),
  
  -- Income - Other
  ('Selling Items', NULL, '#58D68D', 'Income - Other', true),
  ('Cashback & Rewards', NULL, '#7DCEA0', 'Income - Other', true),
  ('Government Benefits', NULL, '#A9DFBF', 'Income - Other', true),
  ('Pension & Retirement', NULL, '#D5F4E6', 'Income - Other', true),
  ('Child Support Received', NULL, '#C8E6C9', 'Income - Other', true),
  ('Alimony Received', NULL, '#A8E6CF', 'Income - Other', true),
  ('Gift Money', NULL, '#58D68D', 'Income - Other', true),
  ('Tax Refund', NULL, '#7DCEA0', 'Income - Other', true),
  ('Other Income', NULL, '#D5F4E6', 'Income - Other', true),
  
  -- Transfers
  ('Account Transfer', NULL, '#95A5A6', 'Transfers', true),
  ('Transfer', NULL, '#BDC3C7', 'Transfers', true)
ON CONFLICT DO NOTHING;

