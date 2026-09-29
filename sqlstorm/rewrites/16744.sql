SELECT u.DisplayName, p.Title, p.CreationDate, t.TagName
FROM Posts p
JOIN Users u ON p.OwnerUserId = u.Id
JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%'
WHERE p.PostTypeId = 1
ORDER BY p.CreationDate DESC, p.Id, t.TagName
LIMIT 10;
