SELECT p.Title, u.DisplayName, COUNT(c.Id) AS CommentCount
FROM Posts p
JOIN Users u ON p.OwnerUserId = u.Id
LEFT JOIN Comments c ON p.Id = c.PostId
WHERE p.PostTypeId = 1  
GROUP BY p.Title, u.DisplayName
ORDER BY CommentCount DESC, p.Title, u.DisplayName
LIMIT 10;
