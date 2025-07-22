// 时区转换工具函数

// 常用时区列表
export const TIMEZONE_OPTIONS = [
	{ value: 'Asia/Shanghai', label: '北京时间 (UTC+8)' },
	{ value: 'UTC', label: '协调世界时 (UTC+0)' },
	{ value: 'America/New_York', label: '纽约时间 (UTC-5/-4)' },
	{ value: 'America/Los_Angeles', label: '洛杉矶时间 (UTC-8/-7)' },
	{ value: 'Europe/London', label: '伦敦时间 (UTC+0/+1)' },
	{ value: 'Europe/Paris', label: '巴黎时间 (UTC+1/+2)' },
	{ value: 'Asia/Tokyo', label: '东京时间 (UTC+9)' },
	{ value: 'Asia/Seoul', label: '首尔时间 (UTC+9)' },
	{ value: 'Australia/Sydney', label: '悉尼时间 (UTC+10/+11)' },
	{ value: 'Asia/Dubai', label: '迪拜时间 (UTC+4)' },
	{ value: 'Asia/Singapore', label: '新加坡时间 (UTC+8)' },
	{ value: 'Asia/Hong_Kong', label: '香港时间 (UTC+8)' },
	{ value: 'Asia/Taipei', label: '台北时间 (UTC+8)' }
];

// 默认时区
export const DEFAULT_TIMEZONE = 'Asia/Shanghai';

// 获取当前设置的时区
export function getCurrentTimezone(): string {
	if (typeof window !== 'undefined') {
		return localStorage.getItem('timezone') || DEFAULT_TIMEZONE;
	}
	return DEFAULT_TIMEZONE;
}

// 设置时区
export function setTimezone(timezone: string): void {
	if (typeof window !== 'undefined') {
		localStorage.setItem('timezone', timezone);
	}
}

// 格式化时间戳到指定时区
export function formatTimestamp(
	timestamp: string | number | Date,
	timezone: string = getCurrentTimezone(),
	format: 'datetime' | 'date' | 'time' = 'datetime'
): string {
	try {
		let date: Date;

		if (typeof timestamp === 'string') {
			// 处理字符串时间戳
			date = new Date(timestamp);
		} else if (typeof timestamp === 'number') {
			// 处理数字时间戳（秒或毫秒）
			date = new Date(timestamp < 1e12 ? timestamp * 1000 : timestamp);
		} else {
			date = timestamp;
		}

		// 检查日期是否有效
		if (isNaN(date.getTime())) {
			return '无效时间';
		}

		const options: Intl.DateTimeFormatOptions = {
			timeZone: timezone,
			year: 'numeric',
			month: '2-digit',
			day: '2-digit',
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit',
			hour12: false
		};

		switch (format) {
			case 'date':
				delete options.hour;
				delete options.minute;
				delete options.second;
				break;
			case 'time':
				delete options.year;
				delete options.month;
				delete options.day;
				break;
		}

		return new Intl.DateTimeFormat('zh-CN', options).format(date);
	} catch (error) {
		console.error('时间格式化失败:', error);
		return '格式化失败';
	}
}

// 获取相对时间描述
export function getRelativeTime(
	timestamp: string | number | Date,
	timezone: string = getCurrentTimezone()
): string {
	try {
		let date: Date;

		if (typeof timestamp === 'string') {
			date = new Date(timestamp);
		} else if (typeof timestamp === 'number') {
			date = new Date(timestamp < 1e12 ? timestamp * 1000 : timestamp);
		} else {
			date = timestamp;
		}

		if (isNaN(date.getTime())) {
			return '无效时间';
		}

		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffSeconds = Math.floor(diffMs / 1000);
		const diffMinutes = Math.floor(diffSeconds / 60);
		const diffHours = Math.floor(diffMinutes / 60);
		const diffDays = Math.floor(diffHours / 24);

		if (diffSeconds < 60) {
			return '刚刚';
		} else if (diffMinutes < 60) {
			return `${diffMinutes}分钟前`;
		} else if (diffHours < 24) {
			return `${diffHours}小时前`;
		} else if (diffDays < 7) {
			return `${diffDays}天前`;
		} else {
			// 超过一周显示具体日期
			return formatTimestamp(date, timezone, 'datetime');
		}
	} catch (error) {
		console.error('相对时间计算失败:', error);
		return '计算失败';
	}
}

// 转换UTC时间到指定时区
export function convertUTCToTimezone(
	utcTimestamp: string | number | Date,
	_timezone: string = getCurrentTimezone() // eslint-disable-line @typescript-eslint/no-unused-vars
): Date {
	let date: Date;

	if (typeof utcTimestamp === 'string') {
		// 如果字符串不包含时区信息，假设为UTC
		if (!utcTimestamp.includes('Z') && !utcTimestamp.includes('+') && !utcTimestamp.includes('-')) {
			date = new Date(utcTimestamp + 'Z');
		} else {
			date = new Date(utcTimestamp);
		}
	} else if (typeof utcTimestamp === 'number') {
		date = new Date(utcTimestamp < 1e12 ? utcTimestamp * 1000 : utcTimestamp);
	} else {
		date = utcTimestamp;
	}

	return date;
}

// 获取时区偏移信息
export function getTimezoneOffset(timezone: string): string {
	try {
		const now = new Date();
		const utc = new Date(now.getTime() + now.getTimezoneOffset() * 60000);
		const targetTime = new Date(utc.toLocaleString('en-US', { timeZone: timezone }));
		const offset = (targetTime.getTime() - utc.getTime()) / (1000 * 60 * 60);

		const sign = offset >= 0 ? '+' : '-';
		const hours = Math.floor(Math.abs(offset));
		const minutes = Math.floor((Math.abs(offset) - hours) * 60);

		return `UTC${sign}${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}`;
	} catch {
		return 'UTC+00:00';
	}
}
