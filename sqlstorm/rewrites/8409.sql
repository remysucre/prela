
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        u.DisplayName AS OwnerDisplayName,
        p.CreationDate,
        COUNT(c.Id) AS CommentCount,
        COUNT(DISTINCT v.Id) AS VoteCount,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS OwnerPostRank
    FROM Posts p
    LEFT JOIN Users u ON p.OwnerUserId = u.Id
    LEFT JOIN Comments c ON p.Id = c.PostId
    LEFT JOIN Votes v ON p.Id = v.PostId
    WHERE p.CreationDate >= '2022-01-01'
    GROUP BY p.Id, p.Title, u.DisplayName, p.CreationDate, p.OwnerUserId
), FilteredPosts AS (
    SELECT 
        PostId,
        Title,
        OwnerDisplayName,
        CreationDate,
        CommentCount,
        VoteCount
    FROM RankedPosts
    WHERE OwnerPostRank <= 5
)
SELECT 
    fp.PostId,
    fp.Title,
    fp.OwnerDisplayName,
    fp.CreationDate,
    fp.CommentCount,
    fp.VoteCount,
    t.TagName,
    pt.Name AS PostTypeName,
    COALESCE(b.BadgeCount, 0) AS BadgeCount
FROM FilteredPosts fp
LEFT JOIN Posts p ON fp.PostId = p.Id
LEFT JOIN Tags t ON t.ExcerptPostId = p.Id
LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
LEFT JOIN (
    SELECT 
        UserId, 
        COUNT(*) AS BadgeCount 
    FROM Badges 
    GROUP BY UserId
) b ON p.OwnerUserId = b.UserId
ORDER BY fp.CreationDate DESC, fp.VoteCount DESC;
