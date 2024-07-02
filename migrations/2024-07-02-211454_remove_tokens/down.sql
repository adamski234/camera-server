-- This file should undo anything in `up.sql`
CREATE TABLE `authenticationtoken`(
	`token` TEXT NOT NULL PRIMARY KEY,
	`user_id` BINARY NOT NULL,
	`expired_date` TIMESTAMP NOT NULL,
	FOREIGN KEY (`user_id`) REFERENCES `users`(`user_id`)
);