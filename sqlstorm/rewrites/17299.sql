SELECT p.Id AS PostId, p.Title, u.DisplayName AS Owner, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, t.TagName
FROM Posts p
JOIN Users u ON p.OwnerUserId = u.Id
LEFT JOIN Tags t ON p.Tags LIKE CONCAT('%', t.TagName, '%')
WHERE p.PostTypeId = 1
ORDER BY p.CreationDate DESC, p.Id, t.TagName
LIMIT 10;
