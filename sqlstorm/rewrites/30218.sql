WITH RECURSIVE UserPosts AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(p.Id) AS TotalPosts,
        SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
        SUM(p.ViewCount) AS TotalViews
    FROM Users u
    LEFT JOIN Posts p ON u.Id = p.OwnerUserId
    GROUP BY u.Id, u.DisplayName
),
RecentActivity AS (
    SELECT 
        p.Id AS PostId,
        p.OwnerUserId,
        p.Title,
        p.CreationDate,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS ActivityRank
    FROM Posts p
    WHERE p.CreationDate > (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
),
TopTags AS (
    SELECT 
        t.TagName,
        COUNT(p.Id) AS TagCount
    FROM Tags t
    JOIN Posts p ON p.Tags ILIKE '%' || t.TagName || '%'
    GROUP BY t.TagName
    ORDER BY TagCount DESC
    LIMIT 10
)
SELECT 
    u.DisplayName,
    u.Reputation,
    COALESCE(up.TotalPosts, 0) AS TotalPosts,
    COALESCE(up.PositivePosts, 0) AS PositivePosts,
    COALESCE(up.NegativePosts, 0) AS NegativePosts,
    COALESCE(up.TotalViews, 0) AS TotalViews,
    ra.PostId,
    ra.Title,
    ra.CreationDate AS RecentPostDate,
    tt.TagName,
    tt.TagCount
FROM Users u
LEFT JOIN UserPosts up ON u.Id = up.UserId
LEFT JOIN RecentActivity ra ON u.Id = ra.OwnerUserId AND ra.ActivityRank = 1
JOIN TopTags tt ON tt.TagCount > 0
WHERE u.Reputation > 100
ORDER BY u.Reputation DESC, TotalPosts DESC;