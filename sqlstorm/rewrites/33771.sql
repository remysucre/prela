WITH RankedPosts AS (
    SELECT 
        p.Id,
        p.Title,
        p.CreationDate,
        p.ViewCount,
        p.AnswerCount,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Id) AS Rank
    FROM 
        Posts p
    WHERE 
        p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
),
RecentPostHistory AS (
    SELECT 
        ph.Id,
        ph.PostId,
        ph.PostHistoryTypeId,
        ph.CreationDate,
        ph.Comment,
        ph.UserDisplayName,
        p.Title AS PostTitle
    FROM 
        PostHistory ph
    JOIN 
        Posts p ON ph.PostId = p.Id
    WHERE 
        ph.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
),
TopTags AS (
    SELECT 
        t.TagName,
        COUNT(*) AS TagCount
    FROM 
        Tags t
    JOIN 
        Posts p ON t.ExcerptPostId = p.Id
    GROUP BY 
        t.TagName
    ORDER BY
        TagCount DESC, t.TagName
    LIMIT 5
),
UserActivity AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        COUNT(DISTINCT p.Id) AS PostsCreated
    FROM 
        Users u
    LEFT JOIN 
        Votes v ON u.Id = v.UserId
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    WHERE 
        u.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
    GROUP BY 
        u.Id, u.DisplayName
),
FinalResults AS (
    SELECT 
        r.Id AS PostId,
        r.Title,
        r.CreationDate,
        r.ViewCount,
        r.AnswerCount,
        ph.UserDisplayName AS LastEditor,
        T.TagName,
        ua.DisplayName AS UserName,
        ua.UpVotes,
        ua.DownVotes,
        ua.PostsCreated
    FROM 
        RankedPosts r
    LEFT JOIN 
        RecentPostHistory ph ON r.Id = ph.PostId
    LEFT JOIN 
        TopTags T ON POSITION(T.TagName IN r.Title) > 0
    LEFT JOIN 
        UserActivity ua ON r.Id = ua.UserId
    WHERE 
        r.Rank <= 10
)
SELECT 
    PostId,
    Title,
    CreationDate,
    ViewCount,
    AnswerCount,
    LastEditor,
    TagName,
    UserName,
    UpVotes,
    DownVotes,
    PostsCreated
FROM 
    FinalResults
WHERE 
    UserName IS NOT NULL OR TagName IS NOT NULL
ORDER BY 
    CreationDate DESC;