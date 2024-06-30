-- Your SQL goes here
CREATE TABLE `authenticationtoken`(
	`token` TEXT NOT NULL PRIMARY KEY,
	`user_id` BINARY NOT NULL,
	`expired_date` TIMESTAMP NOT NULL,
	FOREIGN KEY (`user_id`) REFERENCES `users`(`user_id`)
);

CREATE TABLE `device`(
	`device_id` BINARY NOT NULL PRIMARY KEY,
	`mac_address` BINARY NOT NULL,
	`auth_key` BINARY NOT NULL,
	`registration_first_stage` BOOL NOT NULL,
	`user_id` BINARY NOT NULL,
	FOREIGN KEY (`user_id`) REFERENCES `users`(`user_id`)
);

CREATE TABLE `users`(
	`user_id` BINARY NOT NULL PRIMARY KEY,
	`username` TEXT NOT NULL UNIQUE,
	`password` TEXT NOT NULL,
	`email` TEXT NOT NULL UNIQUE
);

